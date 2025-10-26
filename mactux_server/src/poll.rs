use crossbeam::channel::{Receiver, Select};
use std::{collections::HashMap, time::Duration};
use structures::io::PollEvents;

#[derive(Debug)]
pub struct PollSet {
    select: Select<'static>,
    tokens: HashMap<usize, Box<PollToken>>,
}
impl PollSet {
    pub fn new() -> Self {
        Self {
            select: Select::new(),
            tokens: HashMap::new(),
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

#[derive(Debug)]
pub struct PollToken {
    vfd: u64,
    interest: PollEvents,
    receiver: Receiver<PollEvents>,
}
impl PollToken {
    pub fn ready(&self, latest: PollEvents) -> bool {
        latest.intersects(self.interest)
    }
}
