//! Infrastructure of interruptible requests.

use crate::{task::process::Process, util::Shared, vfd::PollToken};
use crossbeam::channel::Select;
use rustc_hash::FxHashMap;
use std::{io::Read, os::unix::net::UnixStream, time::Duration};
use structures::{
    error::LxError,
    internal::mactux_ipc::{InterruptibleRequest, Response},
    io::PollEvents,
};

#[derive(Debug)]
pub struct InterruptibleSession {
    stream: UnixStream,
    req: Option<InterruptibleRequest>,
}
impl InterruptibleSession {
    pub fn new(stream: UnixStream, req: InterruptibleRequest) -> Self {
        Self {
            stream,
            req: Some(req),
        }
    }

    pub fn run(mut self) {
        match self.req.take().unwrap() {
            InterruptibleRequest::VfdPoll(fds, timeout) => self.vfd_poll(fds, timeout),
        }
    }

    fn vfd_poll(self, fds: Vec<(u64, PollEvents)>, timeout: Option<Duration>) {
        let mut poll_set = PollSet::new();
        for (vfd, events) in fds {
            if let Some(vfd_body) = Process::current().vfd.get(vfd)
                && let Ok(mut poll_token) = vfd_body.poll(events)
            {
                poll_token.vfd = vfd;
                poll_set.insert(Box::new(poll_token));
            }
        }
        self.impl_helper(move |terminator| {
            let terminator = poll_set.insert(Box::new(terminator));
            match poll_set.poll(timeout) {
                Some((index, token)) => {
                    if index == terminator {
                        return None;
                    }
                    Some(Response::Poll(Some((token.vfd, token.interest))))
                }
                None => Some(Response::Poll(None)),
            }
        });
    }

    fn impl_helper(self, f: impl FnOnce(PollToken) -> Option<Response> + Send) {
        let (terminator_tx, terminator_rx) = crossbeam::channel::bounded(1);
        let parent = Process::current();
        let apple_pid = Shared::id(&parent);
        let poll_token = PollToken {
            vfd: 0,
            interest: PollEvents::all(),
            receiver: terminator_rx,
        };
        std::thread::scope(|scope| {
            scope.spawn(|| {
                let mut buf = [0; 1];
                _ = (&self.stream).read(&mut buf);
                _ = terminator_tx.send(PollEvents::all());
            });

            scope.spawn(|| {
                let err = crate::task::configure()
                    .parent(parent)
                    .apple_pid(apple_pid as _)
                    .exec()
                    .is_err();
                if err {
                    _ = postcard::to_io(&Response::Error(LxError::EINVAL), &mut (&self.stream));
                    return;
                }
                let Some(resp) = f(poll_token) else {
                    return;
                };
                _ = postcard::to_io(&resp, &mut (&self.stream));
            });
        });
    }
}

#[derive(Debug)]
pub struct PollSet {
    select: Select<'static>,
    tokens: FxHashMap<usize, Box<PollToken>>,
}
impl PollSet {
    pub fn new() -> Self {
        Self {
            select: Select::new(),
            tokens: FxHashMap::default(),
        }
    }

    pub fn insert(&mut self, token: Box<PollToken>) -> usize {
        unsafe {
            let index = self.select.recv(&*&raw const token.receiver);
            self.tokens.insert(index, token);
            index
        }
    }

    pub fn remove(&mut self, index: usize) {
        self.select.remove(index);
        self.tokens.remove(&index);
    }

    pub fn poll(&mut self, timeout: Option<Duration>) -> Option<(usize, &PollToken)> {
        loop {
            let selop = match timeout {
                Some(dur) => self.select.select_timeout(dur),
                None => Ok(self.select.select()),
            }
            .ok()?;
            let index = selop.index();
            let token = self
                .tokens
                .get(&index)
                .expect("inconsistent poll set state");
            let Ok(latest) = selop.recv(&token.receiver) else {
                continue;
            };
            if !token.ready(latest) {
                continue;
            }
            return Some((index, &**token));
        }
    }
}
