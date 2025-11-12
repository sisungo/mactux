use crate::sysinfo::page_size;
use std::io::Write;
use structures::{error::LxError, files::Meminfo};

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
    let mut s = Vec::new();
    for i in std::env::args().skip(1) {
        s.append(&mut i.into_bytes());
        s.push(b' ');
    }
    s.push(b'\n');
    Ok(s)
}
