mod pid;
mod sysinfo;

use crate::filesystem::kernfs::{DirEntry, KernFs, fn_file};
use std::sync::Arc;
use sysinfo::{cmdline, cpuinfo, loadavg, meminfo, uptime};

pub fn empty() -> Arc<KernFs> {
    let kernfs = KernFs::new();
    let mut writer = kernfs.0.table.write().unwrap();

    writer.insert("meminfo".into(), DirEntry::RegularFile(fn_file(meminfo)));
    writer.insert("uptime".into(), DirEntry::RegularFile(fn_file(uptime)));
    writer.insert("loadavg".into(), DirEntry::RegularFile(fn_file(loadavg)));
    writer.insert("cpuinfo".into(), DirEntry::RegularFile(fn_file(cpuinfo)));
    writer.insert("cmdline".into(), DirEntry::RegularFile(fn_file(cmdline)));

    drop(writer);
    Arc::new(kernfs)
}

pub fn add_process(kernfs: &KernFs, apple_pid: libc::pid_t, linux_pid: i32) {
    kernfs.0.table.write().unwrap().insert(
        linux_pid.to_string().into_bytes(),
        DirEntry::Directory(pid::dir(apple_pid)),
    );
}

pub fn del_process(kernfs: &KernFs, linux_pid: i32) {
    kernfs
        .0
        .table
        .write()
        .unwrap()
        .remove(linux_pid.to_string().as_bytes());
}
