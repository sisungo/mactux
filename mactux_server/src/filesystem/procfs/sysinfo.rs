use std::io::Write;
use structures::{
    error::LxError,
    files::{Fstab, FstabEntry},
};

use crate::task::process::Process;

pub fn meminfo() -> Result<Vec<u8>, LxError> {
    let mem_info = crate::sysinfo::MemInfo::acquire()?;

    let mut s = Vec::new();
    writeln!(&mut s, "MemTotal: {} kB", mem_info.total_ram / 1024).unwrap();
    writeln!(&mut s, "MemFree: {} kB", mem_info.free_ram / 1024).unwrap();
    writeln!(&mut s, "MemAvailable: {} kB", mem_info.avail_ram / 1024).unwrap();
    writeln!(&mut s, "Active: {} kB", mem_info.active / 1024).unwrap();
    writeln!(&mut s, "Inactive: {} kB", mem_info.inactive / 1024).unwrap();
    writeln!(&mut s, "SwapTotal: {} kB", mem_info.total_swap / 1024).unwrap();
    writeln!(&mut s, "SwapFree: {} kB", mem_info.free_swap / 1024).unwrap();
    Ok(s)
}

pub fn mounts() -> Result<Vec<u8>, LxError> {
    let mounts = Process::current().mnt.mounts();
    let mut fstab = Fstab(Vec::with_capacity(mounts.len()));

    for mount in mounts {
        fstab.0.push(FstabEntry {
            device: String::from_utf8_lossy(&mount.source).to_string(),
            mount_point: String::from_utf8_lossy(&mount.mountpoint.express()).to_string(),
            fs_type: mount.filesystem.fs_type().into(),
            options: "defaults".into(),
            dump: 0,
            pass: 0,
        });
    }

    Ok(fstab.to_string().into_bytes())
}

pub fn uptime() -> Result<Vec<u8>, LxError> {
    Ok(format!("{} 0", crate::sysinfo::sysinfo()?.uptime).into_bytes())
}

pub fn loadavg() -> Result<Vec<u8>, LxError> {
    Err(LxError::EINVAL)
}

pub fn cpuinfo() -> Result<Vec<u8>, LxError> {
    Err(LxError::EINVAL)
}

pub fn stat() -> Result<Vec<u8>, LxError> {
    let mut s = Vec::new();

    let user = 0;
    let nice = 0;
    let system = 0;
    let idle = 0;
    let iowait = 0;
    let irq = 0;
    let softirq = 0;

    writeln!(
        &mut s,
        "cpu {user} {nice} {system} {idle} {iowait} {irq} {softirq}"
    )
    .unwrap();

    Ok(s)
}

pub fn cmdline() -> Result<Vec<u8>, LxError> {
    Ok(Vec::new())
}
