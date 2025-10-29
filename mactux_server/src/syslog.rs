use crossbeam::atomic::AtomicCell;
use std::{num::NonZero, sync::mpsc, thread::available_parallelism};
use structures::misc::LogLevel;

/// The system logging module.
#[derive(Debug)]
pub struct Syslog {
    tx: mpsc::SyncSender<Message>,
    console_loglevel: AtomicCell<LogLevel>,
    record_loglevel: AtomicCell<LogLevel>,
}
impl Syslog {
    pub fn new() -> Self {
        let capacity = available_parallelism().map(NonZero::get).unwrap_or(8) * 32;
        let (tx, rx) = mpsc::sync_channel(capacity);
        let syslog_impl = SyslogImpl { rx };
        syslog_impl.start();
        Self {
            tx,
            console_loglevel: AtomicCell::new(LogLevel::KERN_WARNING),
            record_loglevel: AtomicCell::new(LogLevel::KERN_DEBUG),
        }
    }
}

/// Implementation of the system logging thread.
#[derive(Debug)]
struct SyslogImpl {
    rx: mpsc::Receiver<Message>,
}
impl SyslogImpl {
    fn run(self) {
        while let Ok(msg) = self.rx.recv() {}
    }

    fn start(self) {
        std::thread::Builder::new()
            .name(String::from("Syslog"))
            .spawn(move || self.run())
            .expect("failed to start syslog thread");
    }
}

/// A message sent to the syslog thread.
#[derive(Debug)]
enum Message {
    WriteLog(),
}
