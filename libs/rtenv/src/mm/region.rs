//! Motivated memory region tracking system.
//!
//! Introducing this system helps us implement advanced memory management features that is absent in macOS, for example,
//! `mremap()` on non-anonymous memory, `mmap()` on virtual file descriptors, etc.

#[derive(Debug)]
pub struct MemoryTracker {}
impl MemoryTracker {
    pub fn new() -> Self {
        Self {}
    }
}
