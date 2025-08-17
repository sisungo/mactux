use std::{io::Write, sync::Arc};
use tracing_subscriber::fmt::MakeWriter;

#[derive(Debug, Clone)]
pub struct Syslog(Arc<SyslogInner>);
impl MakeWriter<'_> for Syslog {
    type Writer = Self;

    fn make_writer(&self) -> Self::Writer {
        self.clone()
    }
}
impl Write for Syslog {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        (&*self.0).write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        (&*self.0).flush()
    }
}

#[derive(Debug)]
struct SyslogInner {}
impl Write for &SyslogInner {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        todo!()
    }

    fn flush(&mut self) -> std::io::Result<()> {
        todo!()
    }
}
