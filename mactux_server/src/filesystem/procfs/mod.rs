mod pid;
mod sysinfo;

use crate::{
    filesystem::kernfs::{DirEntry, KernFs, fn_file},
    filesystem::vfs::Mountable,
};
use std::sync::Arc;
use structures::error::LxError;
use sysinfo::{cmdline, cpuinfo, loadavg, meminfo, uptime};

pub fn mountable() -> Result<Arc<dyn Mountable>, LxError> {
    let kernfs = KernFs::new();
    let mut writer = kernfs.0.table.write().unwrap();

    writer.insert("meminfo".into(), DirEntry::RegularFile(fn_file(meminfo)));
    writer.insert("uptime".into(), DirEntry::RegularFile(fn_file(uptime)));
    writer.insert("loadavg".into(), DirEntry::RegularFile(fn_file(loadavg)));
    writer.insert("cpuinfo".into(), DirEntry::RegularFile(fn_file(cpuinfo)));
    writer.insert("cmdline".into(), DirEntry::RegularFile(fn_file(cmdline)));

    drop(writer);
    Ok(Arc::new(kernfs))
}
