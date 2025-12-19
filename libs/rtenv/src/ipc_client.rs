//! Client implementation of the MacTux IPC protocol.

use crate::{
    posix_num, process, thread,
    util::{ipc_fail, posix_result},
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
use structures::{
    error::LxError,
    fs::{Dirent64, StatFs, Statx},
    internal::mactux_ipc::*,
    misc::SysInfo,
};

pub fn call_server<T: FromResponse>(req: Request) -> T {
    with_client(
        |client| match T::from_response(client.invoke(req).unwrap()) {
            Some(x) => x,
            None => ipc_fail(),
        },
    )
}

/// A MacTux IPC client.
#[derive(Debug)]
pub struct Client(UnixStream);
impl Client {
    /// Enables close-on-exec for this client.
    pub fn enable_cloexec(&self) -> Result<(), LxError> {
        let fd = self.0.as_raw_fd();
        crate::io::set_cloexec(fd)?;
        Ok(())
    }

    /// Disables close-on-exec for this client.
    pub fn disable_cloexec(&self) -> Result<(), LxError> {
        let fd = self.0.as_raw_fd();
        let original: i32 = unsafe { posix_num!(libc::fcntl(fd, libc::F_GETFD)) }?;
        unsafe {
            posix_result(libc::fcntl(
                fd,
                libc::F_SETFD,
                (original & !libc::FD_CLOEXEC) as usize,
            ))
        }?;
        Ok(())
    }

    /// Forces a handshake message.
    pub fn force_handshake(&self) {
        let mut buf = postcard::to_stdvec(&HandshakeRequest::new())
            .expect("all handshake requests should be valid postcard");
        self.send(&buf).unwrap();
        self.recv(&mut buf).unwrap();
        let response: HandshakeResponse = postcard::from_bytes(&buf).expect("forced handshake");
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
        (&self.0).write_all(buf)?;

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
            thread::with_context(|ctx| {
                let mut buf = ctx.ipc_buf.borrow_mut();
                buf.clear();
                postcard::to_io(&req, &mut *buf).expect("all requests should be valid postcard");
                self.send(&buf)?;
                self.recv(&mut buf)?;
                postcard::from_bytes(&buf).map_err(|err| {
                    std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        format!("failed to deserialize response: {err}"),
                    )
                })
            })
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
        let mut buf = Vec::new();
        self.0
            .read_to_end(&mut buf)
            .expect("unexpected end of file");
        postcard::from_bytes(&buf).expect("failed to deserialize response")
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
    let buf = postcard::to_stdvec(&Request::CallInterruptible(ireq))
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

pub trait FromResponse
where
    Self: Sized,
{
    fn from_response(resp: Response) -> Option<Self>;
}
impl<T> FromResponse for Result<T, LxError>
where
    T: FromResponse,
{
    fn from_response(resp: Response) -> Option<Self> {
        match resp {
            Response::Error(err) => Some(Err(err)),
            other => T::from_response(other).map(Ok),
        }
    }
}
impl FromResponse for () {
    fn from_response(resp: Response) -> Option<Self> {
        #[cfg(debug_assertions)]
        if !matches!(resp, Response::Nothing) {
            return None;
        }

        Some(())
    }
}
impl<T> FromResponse for Option<T>
where
    T: FromResponse,
{
    fn from_response(resp: Response) -> Option<Self> {
        match resp {
            Response::Nothing => Some(None),
            other => T::from_response(other).map(Some),
        }
    }
}
impl FromResponse for NetworkNames {
    fn from_response(resp: Response) -> Option<Self> {
        match resp {
            Response::NetworkNames(x) => Some(x),
            _ => None,
        }
    }
}
impl FromResponse for SysInfo {
    fn from_response(resp: Response) -> Option<Self> {
        match resp {
            Response::SysInfo(x) => Some(*x),
            _ => None,
        }
    }
}
impl FromResponse for Dirent64 {
    fn from_response(resp: Response) -> Option<Self> {
        match resp {
            Response::Dirent64(x) => Some(x),
            _ => None,
        }
    }
}
impl FromResponse for Statx {
    fn from_response(resp: Response) -> Option<Self> {
        match resp {
            Response::Stat(x) => Some(*x),
            _ => None,
        }
    }
}
impl FromResponse for StatFs {
    fn from_response(resp: Response) -> Option<Self> {
        match resp {
            Response::StatFs(x) => Some(*x),
            _ => None,
        }
    }
}
