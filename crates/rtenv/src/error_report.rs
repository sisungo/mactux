use std::io::Write;

/// The error report port.
///
/// This is basically the same as [`std::io::Stderr`], but this guarantees that this is mapped to the POSIX primitives
/// directly and no thread-local variables or locks are used, so this can be used from the emulated context or async signal
/// handlers.
#[derive(Debug, Clone, Copy)]
pub struct ErrorReport;
impl Write for ErrorReport {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        unsafe {
            match libc::write(libc::STDERR_FILENO, buf.as_ptr().cast(), buf.len()) {
                -1 => Err(std::io::Error::last_os_error()),
                n => Ok(n as _),
            }
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Prints register information to [`ErrorReport`].
#[cfg(target_arch = "x86_64")]
pub fn print_registers(ctx: &libc::ucontext_t) {
    let thrd_state = unsafe { &(*ctx.uc_mcontext).__ss };
    _ = writeln!(
        ErrorReport,
        "  rax=0x{:016x}, rbx=0x{:016x}, rcx=0x{:016x}, rdx=0x{:016x},",
        thrd_state.__rax, thrd_state.__rbx, thrd_state.__rcx, thrd_state.__rdx
    );
    _ = writeln!(
        ErrorReport,
        "   r8=0x{:016x},  r9=0x{:016x}, r10=0x{:016x}, r11=0x{:016x},",
        thrd_state.__r8, thrd_state.__r9, thrd_state.__r10, thrd_state.__r11
    );
    _ = writeln!(
        ErrorReport,
        "  r12=0x{:016x}, r13=0x{:016x}, r14=0x{:016x}, r15=0x{:016x},",
        thrd_state.__r12, thrd_state.__r13, thrd_state.__r14, thrd_state.__r15
    );
    _ = writeln!(
        ErrorReport,
        "  rip=0x{:016x}, rsp=0x{:016x}, rbp=0x{:016x}, rsi=0x{:016x},",
        thrd_state.__rip, thrd_state.__rsp, thrd_state.__rbp, thrd_state.__rsi
    );
    _ = writeln!(ErrorReport, "  rdi=0x{:016x},", thrd_state.__rdi,);
    _ = writeln!(
        ErrorReport,
        "  rfl=0x{:016x},  cs=0x{:016x},  fs=0x{:016x},  gs=0x{:016x},",
        thrd_state.__rflags, thrd_state.__cs, thrd_state.__fs, thrd_state.__gs
    );
}

/// Makes the process fail immediately.
#[cold]
pub fn fast_fail() -> ! {
    unsafe {
        libc::_exit(101);
    }
}
