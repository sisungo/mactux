/// Minimal TID that indicates a non-main thread rather than a process (or, the "main thread").
pub const TID_MIN: i32 = 0x10000000;

/// Maximum TID.
pub const TID_MAX: i32 = 0x3ffff000;

pub fn is_tid(pid: i32) -> bool {
    (TID_MIN..=TID_MAX).contains(&pid)
}
