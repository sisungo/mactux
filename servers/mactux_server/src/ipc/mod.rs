pub mod interruptible;
pub mod listener;
pub mod methods;
pub mod session;

pub use listener::Listener;

use bincode::{Decode, Encode};
use mactux_ipc::handshake::{HandshakeRequest, HandshakeResponse};
use std::{
    io::{Read, Write},
    os::unix::net::UnixStream,
};

#[derive(Debug)]
pub struct RegChannel(UnixStream);
impl RegChannel {
    pub fn new(st: UnixStream) -> anyhow::Result<Self> {
        let this = Self(st);
        let mut handshake_buf = Vec::new();
        let handshake_req = this.recv::<HandshakeRequest>(&mut handshake_buf)?;
        if handshake_req != HandshakeRequest::new() {
            return Err(anyhow::anyhow!("invalid handshake request"));
        }
        this.send(&HandshakeResponse::new())?;
        Ok(this)
    }

    pub fn send_bytes(&self, data: &[u8]) -> anyhow::Result<()> {
        let len = (data.len() as u64).to_le_bytes();
        (&self.0).write_all(&len)?;
        (&self.0).write_all(data)?;
        Ok(())
    }

    pub fn recv_bytes(&self, buf: &mut Vec<u8>) -> anyhow::Result<()> {
        let mut len = [0u8; _];
        (&self.0).read_exact(&mut len)?;
        let len = u64::from_le_bytes(len) as usize;
        buf.resize(len, 0);
        (&self.0).read_exact(buf)?;
        Ok(())
    }

    pub fn send<T: Encode>(&self, val: &T) -> anyhow::Result<()> {
        let buf = bincode::encode_to_vec(val, bincode::config::standard())?;
        self.send_bytes(&buf)
    }

    pub fn recv<T: Decode<()>>(&self, buf: &mut Vec<u8>) -> anyhow::Result<T> {
        self.recv_bytes(buf)?;
        Ok(bincode::decode_from_slice(buf, bincode::config::standard())?.0)
    }

    pub fn peer_pid(&self) -> Option<libc::pid_t> {
        self.0.peer_cred().ok()?.pid
    }
}
