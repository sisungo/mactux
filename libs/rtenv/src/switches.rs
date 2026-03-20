#[inline]
pub fn ignore_unsupported_syscalls() -> bool {
    matches!(
        std::env::var("MacTux_IgnoreUnsupportedSyscalls").as_deref(),
        Ok("1")
    )
}

#[inline]
pub fn strace() -> bool {
    matches!(std::env::var("MacTux_Strace").as_deref(), Ok("1"))
}
