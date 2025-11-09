use crate::app;
use crossbeam::atomic::AtomicCell;
use std::{
    collections::VecDeque,
    io::Write,
    num::NonZero,
    sync::{
        Arc,
        atomic::{self, AtomicUsize},
        mpsc,
    },
    thread::available_parallelism,
};
use structures::misc::LogLevel;

/// The system logging module.
#[derive(Debug)]
pub struct Syslog {
    tx: mpsc::SyncSender<Request>,
    pub config: Arc<SyslogConfig>,
}
impl Syslog {
    pub fn new() -> Self {
        let capacity = available_parallelism().map(NonZero::get).unwrap_or(8) * 32;
        let (tx, rx) = mpsc::sync_channel(capacity);
        let config = Arc::new(SyslogConfig::new());
        let syslog_impl = SyslogImpl {
            rx,
            config: config.clone(),
            buf: VecDeque::new(),
            buf_used: 0,
        };
        syslog_impl.start();
        Self { tx, config }
    }

    pub fn write(&self, req: WriteLogRequest) {
        if self.config.record_loglevel.load() >= req.level {
            _ = self.tx.send(Request::WriteLog(req));
        }
    }
}

pub fn install_rust() -> Result<(), log::SetLoggerError> {
    log::set_logger(&RustLogger)?;
    log::set_max_level(log::LevelFilter::Trace);
    Ok(())
}

/// Implementation of the system logging thread.
#[derive(Debug)]
struct SyslogImpl {
    rx: mpsc::Receiver<Request>,
    config: Arc<SyslogConfig>,
    buf: VecDeque<Vec<u8>>,
    buf_used: usize,
}
impl SyslogImpl {
    fn run(mut self) {
        while let Ok(msg) = self.rx.recv() {
            match msg {
                Request::WriteLog(req) => self.write_log(req),
            }
        }
    }

    fn start(self) {
        std::thread::Builder::new()
            .name(String::from("Syslog"))
            .spawn(move || self.run())
            .expect("failed to start syslog thread");
    }

    fn write_log(&mut self, req: WriteLogRequest) {
        let mut fmt = Vec::with_capacity(req.content.len() + 16);
        _ = write!(&mut fmt, "[{}] ", timestamp());
        _ = fmt.write_all(&req.content);
        _ = fmt.write_all(b"\n");

        if self.config.console_loglevel.load() >= req.level {
            _ = std::io::stderr().write_all(&fmt);
        }

        self.buf_used += fmt.len();
        self.buf.push_back(fmt);

        if self.buf_used > self.config.buf_size.load(atomic::Ordering::Relaxed) {
            let removed = self.buf.pop_front().map(|x| x.len()).unwrap_or(0);
            self.buf_used -= removed;
        }
    }
}

#[derive(Debug, Clone)]
pub struct WriteLogRequest {
    pub level: LogLevel,
    pub content: Vec<u8>,
}

#[derive(Debug)]
pub struct SyslogConfig {
    pub console_loglevel: AtomicCell<LogLevel>,
    pub record_loglevel: AtomicCell<LogLevel>,
    pub buf_size: AtomicUsize,
}
impl SyslogConfig {
    pub fn new() -> Self {
        Self {
            console_loglevel: AtomicCell::new(LogLevel::KERN_WARNING),
            record_loglevel: AtomicCell::new(LogLevel::KERN_DEBUG),
            buf_size: AtomicUsize::new(8 * 1024),
        }
    }
}

/// A message sent to the syslog thread.
#[derive(Debug)]
enum Request {
    WriteLog(WriteLogRequest),
}

#[derive(Debug)]
struct RustLogger;
impl log::Log for RustLogger {
    fn enabled(&self, _: &log::Metadata) -> bool {
        true
    }

    fn flush(&self) {}

    fn log(&self, record: &log::Record) {
        let level = match record.level() {
            log::Level::Trace => LogLevel::KERN_DEBUG,
            log::Level::Debug => LogLevel::KERN_INFO,
            log::Level::Info => LogLevel::KERN_NOTICE,
            log::Level::Warn => LogLevel::KERN_WARNING,
            log::Level::Error => LogLevel::KERN_ERR,
        };
        let mut module = record.metadata().target();
        if module.is_empty() {
            module = record.module_path().unwrap_or("mactux_server");
        }
        let content = format!("{}: {}", module, record.args());
        app().syslog.write(WriteLogRequest {
            level,
            content: content.into_bytes(),
        });
    }
}

fn timestamp() -> String {
    let mut timespec = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    unsafe {
        // No errors are allowed to return here, so we just keep the fields zero if it has errors.
        _ = libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut timespec);
    }
    let mut time_minor = timespec.tv_nsec.to_string();
    time_minor.truncate(6);
    format!("{:>6}.{:06}", timespec.tv_sec, time_minor)
}
