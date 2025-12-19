//! Infrastructure of interruptible requests.

use crate::{poll::PollToken, task::process::Process, util::Shared};
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

    fn vfd_poll(self, fds: Vec<(u64, PollEvents)>, timeout: Option<Duration>) {}

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
