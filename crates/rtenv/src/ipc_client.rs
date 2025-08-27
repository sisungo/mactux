//! Client implementation of the MacTux IPC protocol.

use crate::{process, thread};
use mactux_ipc::{
    handshake::{HandshakeRequest, HandshakeResponse},
    request::{InterruptibleRequest, Request},
    response::Response,
};
use std::{
    cell::RefCell,
    io::{Read, Write},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::net::UnixStream,
    },
    path::PathBuf,
    sync::Arc,
};

/// A MacTux IPC client.
#[derive(Debug)]
pub struct Client(UnixStream);
impl Client {
    /// Enables close-on-exec for this client.
    pub fn enable_cloexec(&self) -> std::io::Result<()> {
        let fd = self.0.as_raw_fd();
        let original = unsafe {
            match libc::fcntl(fd, libc::F_GETFD) {
                -1 => Err(std::io::Error::last_os_error()),
                n => Ok(n),
            }
        }?;
        let new = original | libc::FD_CLOEXEC;
        unsafe {
            match libc::fcntl(fd, libc::F_SETFD, new) {
                -1 => Err(std::io::Error::last_os_error()),
                _ => Ok(()),
            }
        }?;
        Ok(())
    }

    /// Disables close-on-exec for this client.
    pub fn disable_cloexec(&self) -> std::io::Result<()> {
        let fd = self.0.as_raw_fd();
        let original = unsafe {
            match libc::fcntl(fd, libc::F_GETFD) {
                -1 => Err(std::io::Error::last_os_error()),
                n => Ok(n),
            }
        }?;
        let new = original & !libc::FD_CLOEXEC;
        unsafe {
            match libc::fcntl(fd, libc::F_SETFD, new) {
                -1 => Err(std::io::Error::last_os_error()),
                _ => Ok(()),
            }
        }?;
        Ok(())
    }

    /// Forces a handshake message.
    pub fn force_handshake(&self) {
        let mut buf = bincode::encode_to_vec(&HandshakeRequest::new(), bincode::config::standard())
            .expect("all handshakes should be valid bincode");
        self.send(&buf).unwrap();
        self.recv(&mut buf).unwrap();
        let response: HandshakeResponse =
            bincode::decode_from_slice(&buf, bincode::config::standard())
                .expect("forced handshake")
                .0;
        if response != HandshakeResponse::new() {
            panic!(
                "Server version `{}` does not match client version `{}`",
                response.version,
                HandshakeResponse::new().version
            );
        }
    }

    /// Sends a message.
    pub fn send(&self, buf: &[u8]) -> std::io::Result<()> {
        (&self.0).write_all(&(buf.len() as u64).to_le_bytes())?;
        (&self.0).write_all(&buf)?;

        Ok(())
    }

    /// Receives a message.
    pub fn recv(&self, buf: &mut Vec<u8>) -> std::io::Result<()> {
        let mut len = [0u8; size_of::<u64>()];
        (&self.0).read_exact(&mut len)?;
        let len = u64::from_le_bytes(len);
        buf.clear();
        buf.resize(len as usize, 0);
        (&self.0).read_exact(buf)?;

        Ok(())
    }

    /// Makes an uninterruptible request and waits for its response.
    pub fn invoke(&self, req: Request) -> std::io::Result<Response> {
        crate::signal::without_signals(|| {
            let mut buf = bincode::encode_to_vec(&req, bincode::config::standard())
                .expect("All requests should be valid bincode");
            self.send(&buf)?;
            self.recv(&mut buf)?;
            bincode::decode_from_slice(&buf, bincode::config::standard())
                .map_err(|_| std::io::ErrorKind::Unsupported.into())
                .map(|x| x.0)
        })
    }
}
impl AsRawFd for Client {
    fn as_raw_fd(&self) -> std::os::fd::RawFd {
        self.0.as_raw_fd()
    }
}
impl Drop for Client {
    fn drop(&mut self) {
        process::context()
            .important_fds
            .pin()
            .remove(&self.as_raw_fd());
    }
}

#[derive(Debug)]
pub struct InterruptibleClient(UnixStream);
impl InterruptibleClient {
    pub fn wait(&mut self) -> Response {
        bincode::decode_from_std_read(&mut self.0, bincode::config::standard()).unwrap()
    }

    pub fn interrupt(mut self) {
        _ = self.0.write_all(&[0]);
    }
}
impl AsRawFd for InterruptibleClient {
    fn as_raw_fd(&self) -> std::os::fd::RawFd {
        self.0.as_raw_fd()
    }
}
impl Drop for InterruptibleClient {
    fn drop(&mut self) {
        process::context()
            .important_fds
            .pin()
            .remove(&self.as_raw_fd());
    }
}

/// Sets path of the server socket to connect.
///
/// # Panics
/// This function would panic if there is already a server socket path set.
pub fn set_server_sock_path(val: PathBuf) {
    process::context().server_sock_path.store(Arc::new(val));
}

/// Executes a closure with the thread-local client.
pub fn with_client<T, F: FnOnce(&Client) -> T>(f: F) -> T {
    thread::with_context(|ctx| {
        f(&ctx
            .client
            .get_or_init(|| RefCell::new(make_client()))
            .borrow())
    })
}

/// Creates a client, performing the handshake.
pub fn make_client() -> Client {
    let client = Client(
        UnixStream::connect(&**process::context().server_sock_path.load())
            .expect("unable to connect to MacTux server"),
    );
    client.force_handshake();
    process::context()
        .important_fds
        .pin()
        .insert(client.as_raw_fd());
    client
}

/// Begins an interruptible request.
pub fn begin_interruptible(ireq: InterruptibleRequest) -> InterruptibleClient {
    let client = make_client();
    let buf = bincode::encode_to_vec(
        &Request::CallInterruptible(ireq),
        bincode::config::standard(),
    )
    .expect("All requests should be valid bincode");
    client.send(&buf).unwrap();
    let stream = unsafe { (&raw const client.0).read() };
    std::mem::forget(client);
    InterruptibleClient(stream)
}

/// Updates the thread-local IPC client.
///
/// This is usually used after `fork()` or `clone()` that creates a process (not a thread).
pub fn update_client(client: Client) {
    thread::with_context(|ctx| *ctx.client.get().unwrap().borrow_mut() = client);
}

/// Sets the client file descriptor.
///
/// This is usually used after `execve()`, which inherits the parent client.
pub unsafe fn set_client_fd(fd: libc::c_int) {
    unsafe {
        let client = Client(UnixStream::from_raw_fd(fd));
        _ = client.enable_cloexec();
        client.invoke(Request::AfterExec).unwrap();
        process::context()
            .important_fds
            .pin()
            .insert(client.as_raw_fd());
        thread::with_context(|ctx| ctx.client.set(RefCell::new(client)).unwrap());
    }
}
