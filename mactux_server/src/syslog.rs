use std::collections::VecDeque;

#[derive(Debug)]
pub struct Syslog {
    buf: VecDeque<u8>,
    size: usize,
}
impl Syslog {}
