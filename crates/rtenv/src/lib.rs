pub mod error_report;
pub mod fs;
pub mod io;
pub mod io_uring;
pub mod ipc_client;
pub mod misc;
pub mod mm;
pub mod net;
pub mod process;
pub mod security;
pub mod signal;
pub mod switches;
pub mod sync;
pub mod thread;
pub mod vfd;

#[cfg(target_arch = "x86_64")]
#[path = "emuctx_x86_64.rs"]
pub mod emuctx;

mod util;

pub unsafe fn install() -> std::io::Result<()> {
    unsafe {
        process::install()?;
        thread::install()?;
        signal::install()?;

        Ok(())
    }
}
