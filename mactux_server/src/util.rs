use papaya::Guard;
use rustc_hash::FxBuildHasher;
use std::{
    sync::{
        Arc,
        atomic::{self, AtomicU64},
    },
    time::SystemTime,
};
use structures::time::Timespec;

#[derive(Debug)]
pub struct Registry<T> {
    table: papaya::HashMap<u64, T, FxBuildHasher>,
    next_id: AtomicU64,
}
impl<T: Clone> Registry<T> {
    pub fn new() -> Self {
        Self {
            table: papaya::HashMap::with_capacity_and_hasher(128, FxBuildHasher::default()),
            next_id: AtomicU64::new(1),
        }
    }

    pub fn register(&self, value: T) -> u64 {
        let id = self.next_id.fetch_add(1, atomic::Ordering::Relaxed);
        self.table.pin().insert(id, value);
        id
    }

    pub fn unregister(&self, id: u64, reclaim: bool) {
        let guard = self.table.guard();
        self.table.remove(&id, &guard);
        if reclaim {
            guard.flush();
        }
    }

    pub fn eliminate(&self, mut f: impl FnMut(&T) -> bool) {
        let guard = self.table.guard();
        self.table.retain(|_, v| !f(v), &guard);
        guard.flush();
    }

    pub fn get(&self, id: u64) -> Option<T> {
        self.table.pin().get(&id).cloned()
    }

    pub fn snapshot(&self) -> (Vec<(u64, T)>, u64) {
        (
            self.table
                .pin()
                .iter()
                .map(|(&k, v)| (k, v.clone()))
                .collect(),
            self.next_id.load(atomic::Ordering::Relaxed),
        )
    }

    pub fn from_snapshot(snapshot: (Vec<(u64, T)>, u64)) -> Self {
        let table =
            papaya::HashMap::with_capacity_and_hasher(snapshot.0.len(), FxBuildHasher::default());
        let pinned = table.pin();
        for (k, v) in snapshot.0 {
            pinned.insert(k, v);
        }
        drop(pinned);
        Self {
            table,
            next_id: AtomicU64::new(snapshot.1),
        }
    }
}
impl<T> Registry<Arc<T>> {
    pub fn gc(&self) {
        self.eliminate(|v| Arc::strong_count(v) >= 2);
    }
}
impl<T: Clone> Default for Registry<T> {
    fn default() -> Self {
        Self::new()
    }
}
impl<T: Clone> Clone for Registry<T> {
    fn clone(&self) -> Self {
        Self {
            table: self.table.clone(),
            next_id: AtomicU64::new(self.next_id.load(atomic::Ordering::Relaxed)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileAttrs {
    pub uid: u32,
    pub gid: u32,
    pub mode: u32,
    pub atime: Timespec,
    pub mtime: Timespec,
    pub ctime: Timespec,
}
impl FileAttrs {
    pub fn common() -> FileAttrs {
        FileAttrs {
            uid: 0,
            gid: 0,
            mode: 0o666,
            atime: now(),
            mtime: now(),
            ctime: now(),
        }
    }
}

pub fn now() -> Timespec {
    let now = std::time::SystemTime::now();
    let elapsed = now
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    Timespec {
        tv_sec: elapsed.as_secs() as _,
        tv_nsec: elapsed.subsec_nanos() as _,
    }
}

pub fn c_str(mut rs: Vec<u8>) -> Vec<u8> {
    rs.push(0);
    rs
}
