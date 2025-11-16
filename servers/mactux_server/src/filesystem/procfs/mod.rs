//! Implementation of `procfs`.
//!
//! Actually, it is a special kind of `tmpfs`.

mod pid;
mod sysinfo;

use crate::{
    app,
    filesystem::{
        VPath,
        tmpfs::{DynFile, Tmpfs},
        vfs::{Filesystem, LPath, MakeFilesystem},
    },
    task::{PidNamespace, process::Process},
    util::Shared,
};
use std::sync::Arc;
use structures::{
    error::LxError,
    fs::{FileMode, MountFlags},
};

pub fn new() -> Result<Arc<Tmpfs>, LxError> {
    let tmpfs = Tmpfs::new()?;
    tmpfs.set_fs_type("proc");

    create_dynfile_ro(&tmpfs, "/meminfo", sysinfo::meminfo, 0o444)?;
    create_dynfile_ro(&tmpfs, "/cmdline", sysinfo::cmdline, 0o444)?;
    create_dynfile_ro(&tmpfs, "/cpuinfo", sysinfo::cpuinfo, 0o444)?;
    create_dynfile_ro(&tmpfs, "/loadavg", sysinfo::loadavg, 0o444)?;
    create_dynfile_ro(&tmpfs, "/stat", sysinfo::stat, 0o444)?;
    create_dynfile_ro(&tmpfs, "/uptime", sysinfo::uptime, 0o444)?;
    create_dynfile_ro(&tmpfs, "/filesystems", sysinfo::filesystems, 0o444)?;

    tmpfs.create_dynlink(VPath::parse(b"/self"), || {
        Shared::id(&Process::current()).to_string().into_bytes()
    })?;

    tmpfs.create_dynlink(VPath::parse(b"/mounts"), || b"self/mounts".into())?;

    Ok(tmpfs)
}

pub fn add_proc(tmpfs: &Tmpfs, apple_pid: libc::pid_t, linux_pid: i32) -> Result<(), LxError> {
    create_dir(tmpfs, &format!("/{linux_pid}"), 0o777)?;

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

    create_dir(tmpfs, &format!("/{linux_pid}/task"), 0o777)?;

    Ok(())
}

pub fn del_proc(tmpfs: &Tmpfs, linux_pid: i32) -> Result<(), LxError> {
    tmpfs.rmdir_all(VPath::parse(format!("/{linux_pid}").as_bytes()))
}

pub fn add_thread(
    ns: &dyn PidNamespace,
    tmpfs: &Tmpfs,
    native_tid: libc::pid_t,
) -> Result<(), LxError> {
    let (linux_pid, linux_tid) = thread_linux_ids(ns, native_tid)?;
    create_dir(tmpfs, &format!("/{linux_pid}/task/{linux_tid}"), 0o777)?;
    Ok(())
}

pub fn del_thread(tmpfs: &Tmpfs, linux_pid: i32, linux_tid: i32) -> Result<(), LxError> {
    tmpfs.rmdir_all(VPath::parse(
        format!("/{linux_pid}/task/{linux_tid}").as_bytes(),
    ))
}

pub struct MakeProcfs;
impl MakeFilesystem for MakeProcfs {
    fn make_filesystem(
        &self,
        _: &[u8],
        _: MountFlags,
        _: &[u8],
    ) -> Result<Arc<dyn Filesystem>, LxError> {
        Process::current().pid.procfs()
    }

    fn is_nodev(&self) -> bool {
        true
    }
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

fn create_dir(tmpfs: &Tmpfs, path: &str, permbits: u16) -> Result<(), LxError> {
    let lpath = LPath {
        mountpoint: VPath::parse(b"/"),
        relative: VPath::parse(path.as_bytes()),
    };
    tmpfs.mkdir(lpath, FileMode(permbits))
}

fn thread_linux_ids(ns: &dyn PidNamespace, native_tid: i32) -> Result<(i32, i32), LxError> {
    let thread = app().threads.get(native_tid as _).ok_or(LxError::ESRCH)?;
    let native_pid = Shared::id(&thread.process) as libc::pid_t;
    let linux_tid = ns.ntol(native_tid)?;
    let linux_pid = ns.ntol(native_pid)?;
    Ok((linux_pid, linux_tid))
}
