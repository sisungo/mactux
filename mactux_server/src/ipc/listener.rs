use std::{os::unix::net::UnixListener, path::Path};

pub struct Listener(UnixListener);
impl Listener {
    pub fn new<P: AsRef<Path>>(addr: P) -> anyhow::Result<Self> {
        _ = std::fs::remove_file(addr.as_ref());
        Ok(Self(UnixListener::bind(addr)?))
    }

    pub fn run(self) {
        loop {
            match self.0.accept() {
                Ok((stream, addr)) => {
                    let Ok(sess) = super::session::RegSession::new(stream) else {
                        tracing::warn!("failed to handshake connection at {addr:?}");
                        continue;
                    };
                    if let Err(err) = sess.start() {
                        tracing::warn!("failed to start thread for {addr:?}: {err}");
                        continue;
                    }
                }
                Err(_) => continue,
            }
        }
    }

    pub fn start(self) {
        std::thread::Builder::new()
            .name(String::from("IpcListener"))
            .spawn(|| self.run())
            .expect("failed to spawn thread");
    }
}
