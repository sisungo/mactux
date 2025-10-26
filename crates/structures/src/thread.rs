/// Minimal TID that indicates a non-main thread rather than a process (or, the "main thread").
pub const TID_MIN: i32 = 0x40000000;

/// Maximum TID.
pub const TID_MAX: i32 = 0x7fffffff;
