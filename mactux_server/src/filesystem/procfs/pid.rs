use crate::{
    filesystem::kernfs::{DirEntry, Directory, fn_file},
    util::sysctl_read,
};
use libproc::{bsd_info::BSDInfo, task_info::TaskInfo};
use std::{io::Write, sync::Arc};
use structures::error::LxError;

pub fn dir(native_pid: libc::pid_t) -> Arc<Directory> {
    let directory = Arc::new(Directory::new());
    let mut writer = directory.table.write().unwrap();

    writer.insert(
        "comm".into(),
        DirEntry::RegularFile(fn_file(comm(native_pid))),
    );
    writer.insert(
        "cmdline".into(),
        DirEntry::RegularFile(fn_file(cmdline(native_pid))),
    );
    writer.insert(
        "stat".into(),
        DirEntry::RegularFile(fn_file(stat(native_pid))),
    );

    drop(writer);
    directory
}

pub fn comm(native_pid: libc::pid_t) -> impl Fn() -> Result<Vec<u8>, LxError> + Clone {
    move || {
        let cmdline = parse_mactux_cmdline(apple_cmdline(native_pid)?);
        let Some(arg0) = cmdline.get(0) else {
            return Ok(vec![0]);
        };
        let mut comm = arg0.rsplit(|x| *x == b'/').next().unwrap_or(&[]).to_vec();
        if comm.len() > 15 {
            comm.truncate(15);
        }
        comm.push(0);
        Ok(comm)
    }
}

pub fn cmdline(native_pid: libc::pid_t) -> impl Fn() -> Result<Vec<u8>, LxError> + Clone {
    move || {
        let mut cmdline = parse_mactux_cmdline(apple_cmdline(native_pid)?);
        let mut data = Vec::with_capacity(cmdline.len() * 32);
        for entry in &mut cmdline {
            data.append(entry);
            data.push(0);
        }
        Ok(data)
    }
}

pub fn stat(native_pid: libc::pid_t) -> impl Fn() -> Result<Vec<u8>, LxError> + Clone {
    move || {
        let pid = native_pid;

        let mut comm = comm(native_pid)()?;
        comm.pop();
        let comm = String::from_utf8_lossy(&comm);

        let state = 'R';

        let bsd_info = libproc::proc_pid::pidinfo::<BSDInfo>(native_pid, native_pid as _)
            .map_err(|_| LxError::EPERM)?;
        let ppid = bsd_info.pbi_ppid;
        let pgid = bsd_info.pbi_pgid;
        let start_time = bsd_info.pbi_start_tvusec / 1000;

        let task_info = libproc::proc_pid::pidinfo::<TaskInfo>(native_pid, native_pid as _)
            .map_err(|_| LxError::EPERM)?;
        let min_flt = task_info.pti_faults;
        let cmin_flt = task_info.pti_faults;
        let maj_flt = task_info.pti_faults;
        let cmaj_flt = task_info.pti_faults;
        let utime = task_info.pti_total_user / 1_000_000;
        let stime = task_info.pti_total_system / 1_000_000;
        let cutime = task_info.pti_total_user / 1_000_000;
        let cstime = task_info.pti_total_system / 1_000_000;
        let priority = 0;
        let nice = 0;
        let num_threads = task_info.pti_threadnum;
        let it_real_value = 0;
        let vsize = task_info.pti_virtual_size;
        let rss = task_info.pti_resident_size;

        let mut s = Vec::new();
        write!(&mut s, "{pid} ({comm}) {state} {ppid} {pgid} ").unwrap();
        write!(&mut s, "1 0 0 0 ").unwrap();
        write!(&mut s, "{min_flt} {cmin_flt} {maj_flt} {cmaj_flt} ").unwrap();
        write!(&mut s, "{utime} {stime} {cutime} {cstime} ").unwrap();
        write!(&mut s, "{priority} {nice} ").unwrap();
        write!(&mut s, "{num_threads} ").unwrap();
        write!(&mut s, "{it_real_value} {start_time} ").unwrap();
        write!(&mut s, "{vsize} {rss} 0 ").unwrap();
        write!(&mut s, "0 0 0 0 0 ").unwrap();
        write!(&mut s, "0 0 0 0 ").unwrap();
        write!(&mut s, "0 0 0 ").unwrap();
        write!(&mut s, "17 0 0 0 0 0 0 0 0 0").unwrap();
        Ok(s)
    }
}

fn apple_cmdline(native_pid: libc::pid_t) -> Result<Vec<Vec<u8>>, LxError> {
    let stack = unsafe {
        sysctl_read::<[u8; libc::PROC_PIDPATHINFO_MAXSIZE as _], _>([
            libc::CTL_KERN,
            libc::KERN_PROCARGS2,
            native_pid,
        ])?
    };
    let mut argc = [0; 4];
    argc.copy_from_slice(&stack[..4]);
    let argc = i32::from_ne_bytes(argc) as usize;
    let mut argv = Vec::with_capacity(argc);
    let mut current = Vec::with_capacity(64);
    let mut execpath_skipping = 0;
    for &byte in stack[4..].iter() {
        match execpath_skipping {
            0 => {
                if byte == 0 {
                    execpath_skipping = 1;
                }
                continue;
            }
            1 => {
                if byte != 0 {
                    execpath_skipping = 2;
                    current.push(byte);
                }
                continue;
            }
            2 => (),
            _ => unreachable!(),
        }

        if argv.len() == argc {
            break;
        }
        if byte == 0 {
            argv.push(current.clone());
            current.clear();
            continue;
        }
        current.push(byte);
    }
    Ok(argv)
}

fn parse_mactux_cmdline(apple: Vec<Vec<u8>>) -> Vec<Vec<u8>> {
    let arg0_opt = apple
        .iter()
        .enumerate()
        .find(|(_, v)| *v == b"--arg0")
        .map(|x| x.0);
    let args_sep = apple
        .iter()
        .enumerate()
        .find(|(_, v)| *v == b"--")
        .map(|x| x.0);
    let execfile = apple[..args_sep.unwrap_or(apple.len())]
        .iter()
        .skip(1)
        .fold((false, None), |(flag, data), cur| {
            if data.is_some() {
                return (false, data);
            }
            if flag {
                return (false, None);
            }
            if cur.starts_with(b"--") {
                return (true, None);
            }
            (false, Some(cur))
        })
        .1;
    let arg0 = match arg0_opt {
        Some(x) => apple.get(x + 1).cloned().unwrap_or(Vec::new()),
        None => execfile.cloned().unwrap_or(Vec::new()),
    };
    let mut linux = apple;
    linux
        .extract_if(..args_sep.map(|x| x + 1).unwrap_or(linux.len()), |_| true)
        .for_each(|_| ());
    linux.insert(0, arg0);
    linux
}
