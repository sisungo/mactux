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
    task::{PidNamespace, process::Process, thread::Thread},
    util::Shared,
};
use std::sync::Arc;
use structures::{
    error::LxError,
    fs::{FileMode, FsMagic, MountFlags},
};

pub fn new() -> Result<Arc<Tmpfs>, LxError> {
    let tmpfs = Tmpfs::new()?;
    tmpfs.set_fs_magic(FsMagic::PROC_SUPER_MAGIC);

    create_dynfile_ro(&tmpfs, "/meminfo", sysinfo::meminfo, 0o444)?;
    create_dynfile_ro(&tmpfs, "/cmdline", sysinfo::cmdline, 0o444)?;
    create_dynfile_ro(&tmpfs, "/cpuinfo", sysinfo::cpuinfo, 0o444)?;
    create_dynfile_ro(&tmpfs, "/loadavg", sysinfo::loadavg, 0o444)?;
    create_dynfile_ro(&tmpfs, "/stat", sysinfo::stat, 0o444)?;
    create_dynfile_ro(&tmpfs, "/uptime", sysinfo::uptime, 0o444)?;
    create_dynfile_ro(&tmpfs, "/filesystems", sysinfo::filesystems, 0o444)?;

    tmpfs.create_dynlink(VPath::parse(b"/self"), || {
        current_linux_ids().0.to_string().into_bytes()
    })?;
    tmpfs.create_dynlink(VPath::parse(b"/thread-self"), || {
        let (linux_pid, linux_tid) = current_linux_ids();
        format!("{linux_pid}/task/{linux_tid}").into_bytes()
    })?;

    tmpfs.create_dynlink(VPath::parse(b"/mounts"), || b"self/mounts".into())?;

    Ok(tmpfs)
}

pub fn add_proc(tmpfs: &Tmpfs, apple_pid: libc::pid_t, linux_pid: i32) -> Result<(), LxError> {
    create_dir(tmpfs, &format!("/{linux_pid}"), 0o777)?;
    fill_proc_or_thread(tmpfs, apple_pid, &format!("/{linux_pid}"), false)?;

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
    let path = format!("/{linux_pid}/task/{linux_tid}");
    create_dir(tmpfs, &path, 0o777)?;
    fill_proc_or_thread(tmpfs, native_tid, &path, true)?;
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

fn fill_proc_or_thread(
    tmpfs: &Tmpfs,
    native_pid: i32,
    relpath: &str,
    thread: bool,
) -> Result<(), LxError> {
    create_dynfile_ro(
        tmpfs,
        &format!("{relpath}/cmdline"),
        pid::cmdline(native_pid),
        0o444,
    )?;
    create_dynfile_ro(
        tmpfs,
        &format!("{relpath}/comm"),
        pid::comm(native_pid),
        0o444,
    )?;
    create_dynfile_ro(
        tmpfs,
        &format!("{relpath}/stat"),
        pid::stat(native_pid),
        0o444,
    )?;
    create_dynfile_ro(
        tmpfs,
        &format!("{relpath}/mounts"),
        pid::mounts(native_pid),
        0o444,
    )?;
    create_dynfile_ro(
        tmpfs,
        &format!("{relpath}/environ"),
        pid::environ(native_pid),
        0o444,
    )?;
    create_dynfile_ro(
        tmpfs,
        &format!("{relpath}/statm"),
        pid::statm(native_pid),
        0o444,
    )?;

    if !thread {
        create_dir(tmpfs, &format!("{relpath}/task"), 0o777)?;
    }

    Ok(())
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

fn current_linux_ids() -> (i32, i32) {
    let current = Thread::current();
    let native_pid = Shared::id(&current.process) as i32;
    let linux_pid = current.process.pid.ntol(native_pid).unwrap_or(native_pid);
    let native_tid = Shared::id(&current) as i32;
    let linux_tid = current.process.pid.ntol(native_tid).unwrap_or(native_tid);
    (linux_pid, linux_tid)
}
