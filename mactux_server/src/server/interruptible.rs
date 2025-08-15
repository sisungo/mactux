use crate::{process::ProcessCtx, server::Session};
use mactux_ipc::{request::InterruptibleRequest, response::Response};
use std::{sync::Arc, time::Duration};
use structures::{error::LxError, io::PollEvents};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
    sync::oneshot,
    task::{JoinHandle, JoinSet},
};

pub struct InterruptibleSession {
    conn: UnixStream,
    process: Arc<ProcessCtx>,
}
impl InterruptibleSession {
    pub fn from_session(session: Session) -> Self {
        let conn = unsafe { (&raw const session.conn).read() };
        let process = unsafe { (&raw const session.process).read() };
        std::mem::forget(session);
        Self { conn, process }
    }

    pub async fn run(mut self, ireq: InterruptibleRequest) -> anyhow::Result<()> {
        let mut iop = match ireq {
            InterruptibleRequest::VirtualFdPoll(pollfds, timeout) => {
                self.vfd_poll(pollfds, timeout)
            }
        };
        tokio::select! {
            resp = iop.wait() => {
                self.conn.write_all(&bincode::encode_to_vec(
                    resp.unwrap_or(Response::Error(LxError::EINTR)),
                    bincode::config::standard()
                )?).await?;
            },
            _ = self.conn.read_u8() => {
                iop.interrupt();
            },
        }
        Ok(())
    }
}
impl Drop for InterruptibleSession {
    fn drop(&mut self) {
        crate::process::ctx_close(self.process.native_pid());
    }
}

impl InterruptibleSession {
    fn vfd_poll(&mut self, pollfds: Vec<(u64, u16)>, timeout: Option<Duration>) -> InterruptibleOp {
        let (cancel_token, rx) = oneshot::channel();
        let mut join_set = JoinSet::new();
        for (vfd, interest) in pollfds {
            let Ok(vfd_entity) = self.process.vfd(vfd) else {
                continue; // TODO
            };
            join_set.spawn(async move {
                Ok((
                    vfd,
                    vfd_entity
                        .poll(PollEvents::from_bits_retain(interest))
                        .await?,
                ))
            });
        }
        let timeout = timeout.unwrap_or(Duration::from_secs(u64::MAX));
        let join_handle = tokio::spawn(async move {
            tokio::select! {
                _ = rx => Response::Error(LxError::EINTR),
                _ = tokio::time::sleep(timeout) => Response::Nothing,
                Some(Ok(next)) = join_set.join_next() => match next {
                    Ok((vfd, interest)) => Response::Poll(vfd, interest.bits()),
                    Err(err) => Response::Error(err),
                },
            }
        });
        InterruptibleOp {
            cancel_token,
            join_handle,
        }
    }
}

struct InterruptibleOp {
    cancel_token: oneshot::Sender<()>,
    join_handle: JoinHandle<Response>,
}
impl InterruptibleOp {
    fn interrupt(self) {
        _ = self.cancel_token.send(());
    }

    async fn wait(&mut self) -> Option<Response> {
        (&mut self.join_handle).await.ok()
    }
}
