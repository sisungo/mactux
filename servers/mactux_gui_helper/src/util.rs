use log::{LevelFilter, Log};
use rustc_hash::FxHashMap;

/// A registry for "descriptors".
#[derive(Debug)]
pub struct DescRegistry<T> {
    map: FxHashMap<u64, T>,
    next_id: u64,
}
impl<T> DescRegistry<T> {
    pub fn new() -> Self {
        Self {
            map: FxHashMap::default(),
            next_id: 1,
        }
    }

    pub fn register(&mut self, value: T) -> u64 {
        self.next_id += 1;
        self.map.insert(self.next_id - 1, value);
        self.next_id - 1
    }

    pub fn get(&self, id: u64) -> Option<&T> {
        self.map.get(&id)
    }

    pub fn unregister(&mut self, id: u64) -> Option<T> {
        self.map.remove(&id)
    }
}

struct RustLogger;
impl Log for RustLogger {
    fn enabled(&self, _: &log::Metadata) -> bool {
        true
    }

    fn flush(&self) {}

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        eprintln!(
            "mactux_gui_helper[{}]: {}",
            std::process::id(),
            record.args()
        );
    }
}

pub fn install_logger() {
    log::set_logger(&RustLogger).expect("install_logger is called twice");
    log::set_max_level(LevelFilter::Info);
}
