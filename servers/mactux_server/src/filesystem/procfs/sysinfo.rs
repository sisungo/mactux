use crate::{
    app,
    sysinfo::{boot_time, mach_cpu_core_load_info, mach_host_cpu_load_info, page_size},
    task::thread::Thread,
    util::Shared,
};
use structures::{
    error::LxError,
    files::{Meminfo, ProcLoadavg, ProcStat, ProcStatCpu},
};

pub fn meminfo() -> Result<Vec<u8>, LxError> {
    let apple = crate::sysinfo::MemInfo::acquire()?;
    let linux = Meminfo {
        mem_total: apple.total_ram / 1024,
        mem_free: apple.free_ram() / 1024,
        mem_available: apple.avail_ram() / 1024,
        buffers: 0,
        cached: 0,
        swap_cached: 0,
        swap_free: apple.swap_usage.xsu_avail as usize / 1024,
        swap_total: apple.swap_usage.xsu_total as usize / 1024,
        active: apple.vm_statistics.active_count as usize * page_size() / 1024,
        inactive: apple.vm_statistics.inactive_count as usize * page_size() / 1024,
        active_anon: 0,
        inactive_anon: 0,
        active_file: 0,
        inactive_file: 0,
        unevictable: 0,
        mlocked: 0,
        zswap: 0,
        zswapped: 0,
        dirty: 0,
        writeback: 0,
        anon_pages: 0,
        mapped: 0,
        shmem: 0,
        kreclaimable: 0,
        slab: 0,
        sreclaimable: 0,
        sunreclaim: 0,
        kernel_stack: 0,
        page_tables: 0,
        sec_page_tables: 0,
        nfs_unstable: 0,
        bounce: 0,
        writeback_tmp: 0,
        commit_limit: 0,
        committed_as: 0,
        vmalloc_total: 0,
        vmalloc_used: 0,
        vmalloc_chunk: 0,
        percpu: 0,
        anon_huge_pages: 0,
        shmem_huge_pages: 0,
        shmem_pmd_mapped: 0,
        file_huge_pages: 0,
        file_pmd_mapped: 0,
        huge_pages_total: 0,
        huge_pages_free: 0,
        huge_pages_rsvd: 0,
        huge_pages_surp: 0,
        hugepagesize: 0,
        hugetlb: 0,
        direct_map_4k: 0,
        direct_map_2m: 0,
        direct_map_1g: 0,
    };
    Ok(linux.to_string().into_bytes())
}

pub fn uptime() -> Result<Vec<u8>, LxError> {
    Ok(format!("{} 0", crate::sysinfo::sysinfo()?.uptime).into_bytes())
}

pub fn loadavg() -> Result<Vec<u8>, LxError> {
    let loadavg = ProcLoadavg {
        loadavg: crate::sysinfo::loadavg()?,
        proc_running: 0,
        proc_total: 0,
        last_pid_running: Shared::id(&Thread::current().process()) as _,
    };
    Ok(loadavg.to_string().into_bytes())
}

pub fn cpuinfo() -> Result<Vec<u8>, LxError> {
    Err(LxError::EINVAL)
}

pub fn stat() -> Result<Vec<u8>, LxError> {
    let cpu_overall = stat_cpu_overall()?;
    let mut cpu = vec![cpu_overall];
    cpu.append(&mut stat_cpu_cores()?);
    let stat = ProcStat {
        cpu,
        paged: 0,
        paged_out: 0,
        swap_in: 0,
        swap_out: 0,
        intr: 0,
        ctxt: 0,
        btime: boot_time()?.tv_sec,
        processes: 0,
        procs_running: 0,
        procs_blocked: 0,
    };
    Ok(stat.to_string().into_bytes())
}

pub fn cmdline() -> Result<Vec<u8>, LxError> {
    let mut s = Vec::new();
    for i in std::env::args().skip(1) {
        s.append(&mut i.into_bytes());
        s.push(b' ');
    }
    s.push(b'\n');
    Ok(s)
}

pub fn filesystems() -> Result<Vec<u8>, LxError> {
    Ok(app().filesystems.list().into_bytes())
}

fn stat_cpu_overall() -> Result<ProcStatCpu, LxError> {
    Ok(stat_cpu_from_mach(
        None,
        mach_host_cpu_load_info()?.cpu_ticks,
    ))
}

fn stat_cpu_cores() -> Result<Vec<ProcStatCpu>, LxError> {
    let ncpu: usize = std::thread::available_parallelism()?.into();
    let mach = mach_cpu_core_load_info()?.cast::<[u32; libc::CPU_STATE_MAX as _]>();
    let mut result = Vec::with_capacity(ncpu);
    unsafe {
        let slice = std::slice::from_raw_parts(mach, ncpu);
        for (n, ticks) in slice.iter().enumerate() {
            result.push(stat_cpu_from_mach(Some(n), *ticks));
        }
    }
    Ok(result)
}

fn stat_cpu_from_mach(
    cpu_id: Option<usize>,
    ticks: [u32; libc::CPU_STATE_MAX as _],
) -> ProcStatCpu {
    ProcStatCpu {
        cpu_id,
        user: ticks[libc::CPU_STATE_USER as usize],
        nice: ticks[libc::CPU_STATE_NICE as usize],
        system: ticks[libc::CPU_STATE_SYSTEM as usize],
        idle: ticks[libc::CPU_STATE_IDLE as usize],
        iowait: 0,
        irq: 0,
        softirq: 0,
        steal: 0,
        guest: 0,
        guest_nice: 0,
    }
}
