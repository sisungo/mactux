use crate::filesystem::{
    VPath,
    vfs::{LPath, MountNamespace, NewlyOpen},
};
use dashmap::{DashMap, mapref::entry::Entry};
use rustc_hash::FxBuildHasher;
use std::{
    ffi::{OsString, c_int},
    fmt::Debug,
    ops::Deref,
    os::unix::ffi::OsStringExt,
    path::PathBuf,
    sync::{
        Arc, Weak,
        atomic::{self, AtomicU64},
    },
};
use structures::{error::LxError, fs::OpenHow};

pub struct ReclaimRegistry<T: 'static> {
    table: DashMap<u64, Shared<T>, FxBuildHasher>,
    next_id: AtomicU64,
}
impl<T> ReclaimRegistry<T> {
    pub fn new() -> Self {
        Self {
            table: DashMap::default(),
            next_id: AtomicU64::new(1),
        }
    }

    pub fn intervene(&'static self, id: u64, value: T) -> Shared<T> {
        let shared = Shared {
            registry: self,
            id,
            value: Arc::new(value),
        };
        self.table.insert(id, shared.clone());
        shared
    }

    pub fn tempt<F: FnOnce() -> Result<T, E>, E>(
        &'static self,
        id: u64,
        f: F,
    ) -> Result<(Shared<T>, bool), E> {
        let entry = self.table.entry(id);
        if let Entry::Occupied(occu) = &entry {
            return Ok((occu.get().clone(), false));
        }
        let shared = Shared {
            registry: self,
            id,
            value: Arc::new(f()?),
        };
        Ok((entry.insert(shared).clone(), true))
    }

    pub fn get(&'static self, id: u64) -> Option<Shared<T>> {
        self.table.get(&id).as_deref().cloned()
    }

    pub fn register(&'static self, value: T) -> Shared<T> {
        let id = self.next_id.fetch_add(1, atomic::Ordering::Relaxed);
        let shared = Shared {
            registry: self,
            id,
            value: Arc::new(value),
        };
        self.table.insert(id, shared.clone());
        shared
    }

    pub fn unregister(&self, id: u64) -> Option<Shared<T>> {
        self.table.remove(&id).map(|(_, v)| v)
    }
}

pub struct Shared<T: 'static> {
    registry: &'static ReclaimRegistry<T>,
    id: u64,
    value: Arc<T>,
}
impl<T> Shared<T> {
    pub fn id(this: &Self) -> u64 {
        this.id
    }
}
impl<T> Clone for Shared<T> {
    fn clone(&self) -> Self {
        Self {
            registry: self.registry,
            id: self.id,
            value: self.value.clone(),
        }
    }
}
impl<T> Deref for Shared<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}
impl<T: Debug> Debug for Shared<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Shared")
            .field("id", &self.id)
            .field("value", &self.value)
            .finish()
    }
}
impl<T> Drop for Shared<T> {
    fn drop(&mut self) {
        if Arc::strong_count(&self.value) <= 2 {
            self.registry.unregister(self.id);
        }
    }
}

pub struct WeakShared<T: 'static> {
    registry: &'static ReclaimRegistry<T>,
    id: u64,
    value: Weak<T>,
}
impl<T> WeakShared<T> {
    pub fn id(this: &Self) -> u64 {
        this.id
    }

    pub fn upgrade(&self) -> Option<Shared<T>> {
        self.value.upgrade().map(|v| Shared {
            registry: self.registry,
            id: self.id,
            value: v,
        })
    }
}
impl<T> Clone for WeakShared<T> {
    fn clone(&self) -> Self {
        Self {
            registry: self.registry,
            id: self.id,
            value: self.value.clone(),
        }
    }
}

pub fn symlink_abs(sympath: LPath, symcontent: &[u8]) -> VPath {
    if symcontent.starts_with(b"/") {
        return VPath::parse(symcontent);
    }
    let mut symcontent = VPath::parse(symcontent);
    let mut sympath = sympath.expand();
    sympath.parts.pop();
    sympath.parts.append(&mut symcontent.parts);
    sympath.slash_suffix = symcontent.slash_suffix;
    sympath
}

pub fn test_path(mnt: &MountNamespace, target: &VPath, how: OpenHow) -> bool {
    mnt.locate(target)
        .and_then(|x| x.open(how))
        .map(|x| match x {
            NewlyOpen::Native(np) => PathBuf::from(OsString::from_vec(np)).exists(),
            NewlyOpen::Virtual(_) => true,
        })
        .unwrap_or(false)
}

/// Performs a `sysctl` read operation.
pub unsafe fn sysctl_read<T: Copy, const N: usize>(mut name: [c_int; N]) -> Result<T, LxError> {
    unsafe {
        let mut data: T = std::mem::zeroed();
        let mut size = size_of::<T>();
        match libc::sysctl(
            name.as_mut_ptr(),
            N as _,
            (&raw mut data).cast(),
            &mut size,
            std::ptr::null_mut(),
            0,
        ) {
            -1 => Err(LxError::EINVAL),
            _ => Ok(data),
        }
    }
}
