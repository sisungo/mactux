//! Implementation of `procfs`.
//!
//! Actually, it is a special kind of `tmpfs`.

mod pid;
mod sysinfo;

use crate::{
    filesystem::{
        VPath,
        tmpfs::{DynFile, Tmpfs},
        vfs::{Filesystem, LPath},
    },
    task::process::Process,
    util::Shared,
};
use std::sync::Arc;
use structures::{error::LxError, fs::FileMode};

pub fn new() -> Result<Arc<Tmpfs>, LxError> {
    let tmpfs = Tmpfs::new()?;
    tmpfs.set_fs_type("procfs");

    create_dynfile_ro(&tmpfs, "/meminfo", sysinfo::meminfo, 0o444)?;
    create_dynfile_ro(&tmpfs, "/cmdline", sysinfo::cmdline, 0o444)?;
    create_dynfile_ro(&tmpfs, "/cpuinfo", sysinfo::cpuinfo, 0o444)?;
    create_dynfile_ro(&tmpfs, "/loadavg", sysinfo::loadavg, 0o444)?;
    create_dynfile_ro(&tmpfs, "/stat", sysinfo::stat, 0o444)?;
    create_dynfile_ro(&tmpfs, "/uptime", sysinfo::uptime, 0o444)?;

    tmpfs.create_dynlink(VPath::parse(b"/self"), || {
        Shared::id(&Process::current()).to_string().into_bytes()
    })?;

    tmpfs.create_dynlink(VPath::parse(b"/mounts"), || b"self/mounts".into())?;

    Ok(tmpfs)
}

pub fn add_proc(tmpfs: &Tmpfs, apple_pid: libc::pid_t, linux_pid: i32) -> Result<(), LxError> {
    let lpath = LPath {
        mountpoint: VPath::parse(b"/"),
        relative: VPath::parse(format!("/{linux_pid}").as_bytes()),
    };
    tmpfs.mkdir(lpath, FileMode(0o777))?;

    create_dynfile_ro(
        tmpfs,
        &format!("/{linux_pid}/cmdline"),
        pid::cmdline(apple_pid),
        0o444,
    )?;
    create_dynfile_ro(
        tmpfs,
        &format!("/{linux_pid}/comm"),
        pid::comm(apple_pid),
        0o444,
    )?;
    create_dynfile_ro(
        tmpfs,
        &format!("/{linux_pid}/stat"),
        pid::stat(apple_pid),
        0o444,
    )?;
    create_dynfile_ro(
        tmpfs,
        &format!("/{linux_pid}/mounts"),
        pid::mounts(apple_pid),
        0o444,
    )?;

    Ok(())
}

pub fn del_proc(tmpfs: &Tmpfs, linux_pid: i32) -> Result<(), LxError> {
    tmpfs.rmdir_all(VPath::parse(format!("/{linux_pid}").as_bytes()))
}

fn create_dynfile_ro<R>(tmpfs: &Tmpfs, path: &str, rdf: R, permbits: u16) -> Result<(), LxError>
where
    R: Fn() -> Result<Vec<u8>, LxError> + Send + Sync + 'static,
{
    tmpfs.create_dynfile(
        VPath::parse(path.as_bytes()),
        DynFile::new(rdf, |_| Err(LxError::EIO), permbits),
    )
}
