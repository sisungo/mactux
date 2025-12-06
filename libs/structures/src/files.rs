use std::{fmt::Display, str::FromStr};

/// Structure that represents to `/dev/fstab`.
#[derive(Debug, Clone)]
pub struct Fstab(pub Vec<FstabEntry>);
impl FromStr for Fstab {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let entries = s
            .lines()
            .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
            .map(str::trim)
            .map(FstabEntry::from_str)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Fstab(entries))
    }
}
impl Display for Fstab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for i in self.0.iter() {
            writeln!(f, "{i}")?;
        }
        Ok(())
    }
}

/// An entry in `/dev/fstab`.
#[derive(Debug, Clone)]
pub struct FstabEntry {
    pub device: String,
    pub mount_point: String,
    pub fs_type: String,
    pub options: String,
    pub dump: u32,
    pub pass: u32,
}
impl FromStr for FstabEntry {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split_whitespace();
        let device = parts.next().ok_or(ParseError)?.to_string();
        let mount_point = parts.next().ok_or(ParseError)?.to_string();
        let fs_type = parts.next().ok_or(ParseError)?.to_string();
        let options = parts.next().ok_or(ParseError)?.to_string();
        let dump = parts
            .next()
            .ok_or(ParseError)?
            .parse()
            .map_err(|_| ParseError)?;
        let pass = parts
            .next()
            .ok_or(ParseError)?
            .parse()
            .map_err(|_| ParseError)?;
        Ok(FstabEntry {
            device,
            mount_point,
            fs_type,
            options,
            dump,
            pass,
        })
    }
}
impl Display for FstabEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} {} {} {} {}",
            self.device, self.mount_point, self.fs_type, self.options, self.dump, self.pass
        )
    }
}

/// Structure that represents to `/proc/meminfo`.
///
/// All values are in kilobytes except for explicitly noted.
#[derive(Debug, Clone)]
pub struct Meminfo {
    pub mem_total: usize,
    pub mem_free: usize,
    pub mem_available: usize,
    pub buffers: usize,
    pub cached: usize,
    pub swap_cached: usize,
    pub active: usize,
    pub inactive: usize,
    pub active_anon: usize,
    pub inactive_anon: usize,
    pub active_file: usize,
    pub inactive_file: usize,
    pub unevictable: usize,
    pub mlocked: usize,
    pub swap_total: usize,
    pub swap_free: usize,
    pub zswap: usize,
    pub zswapped: usize,
    pub dirty: usize,
    pub writeback: usize,
    pub anon_pages: usize,
    pub mapped: usize,
    pub shmem: usize,
    pub kreclaimable: usize,
    pub slab: usize,
    pub sreclaimable: usize,
    pub sunreclaim: usize,
    pub kernel_stack: usize,
    pub page_tables: usize,
    pub sec_page_tables: usize,
    pub nfs_unstable: usize,
    pub bounce: usize,
    pub writeback_tmp: usize,
    pub commit_limit: usize,
    pub committed_as: usize,
    pub vmalloc_total: usize,
    pub vmalloc_used: usize,
    pub vmalloc_chunk: usize,
    pub percpu: usize,
    pub anon_huge_pages: usize,
    pub shmem_huge_pages: usize,
    pub shmem_pmd_mapped: usize,
    pub file_huge_pages: usize,
    pub file_pmd_mapped: usize,
    pub huge_pages_total: usize, // unit: N
    pub huge_pages_free: usize,  // unit: N
    pub huge_pages_rsvd: usize,  // unit: N
    pub huge_pages_surp: usize,  // unit: N
    pub hugepagesize: usize,
    pub hugetlb: usize,
    pub direct_map_4k: usize,
    pub direct_map_2m: usize,
    pub direct_map_1g: usize,
}
impl Display for Meminfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "MemTotal:       {:8} kB", self.mem_total)?;
        writeln!(f, "MemFree:        {:8} kB", self.mem_free)?;
        writeln!(f, "MemAvailable:   {:8} kB", self.mem_available)?;
        writeln!(f, "Buffers:        {:8} kB", self.buffers)?;
        writeln!(f, "Cached:         {:8} kB", self.cached)?;
        writeln!(f, "SwapCached:     {:8} kB", self.swap_cached)?;
        writeln!(f, "Active:         {:8} kB", self.active)?;
        writeln!(f, "Inactive:       {:8} kB", self.inactive)?;
        writeln!(f, "Active(anon):   {:8} kB", self.active_anon)?;
        writeln!(f, "Inactive(anon): {:8} kB", self.inactive_anon)?;
        writeln!(f, "Active(file):   {:8} kB", self.active_file)?;
        writeln!(f, "Inactive(file): {:8} kB", self.inactive_file)?;
        writeln!(f, "Unevictable:    {:8} kB", self.unevictable)?;
        writeln!(f, "Mlocked:        {:8} kB", self.mlocked)?;
        writeln!(f, "SwapTotal:      {:8} kB", self.swap_total)?;
        writeln!(f, "SwapFree:       {:8} kB", self.swap_free)?;
        writeln!(f, "Zswap:          {:8} kB", self.zswap)?;
        writeln!(f, "Zswapped:       {:8} kB", self.zswapped)?;
        writeln!(f, "Dirty:          {:8} kB", self.dirty)?;
        writeln!(f, "Writeback:      {:8} kB", self.writeback)?;
        writeln!(f, "AnonPages:      {:8} kB", self.anon_pages)?;
        writeln!(f, "Mapped:         {:8} kB", self.mapped)?;
        writeln!(f, "Shmem:          {:8} kB", self.shmem)?;
        writeln!(f, "KReclaimable:   {:8} kB", self.kreclaimable)?;
        writeln!(f, "Slab:           {:8} kB", self.slab)?;
        writeln!(f, "SReclaimable:   {:8} kB", self.sreclaimable)?;
        writeln!(f, "SUnreclaim:     {:8} kB", self.sunreclaim)?;
        writeln!(f, "KernelStack:    {:8} kB", self.kernel_stack)?;
        writeln!(f, "PageTables:     {:8} kB", self.page_tables)?;
        writeln!(f, "SecPageTables:  {:8} kB", self.sec_page_tables)?;
        writeln!(f, "NFS_Unstable:   {:8} kB", self.nfs_unstable)?;
        writeln!(f, "Bounce:         {:8} kB", self.bounce)?;
        writeln!(f, "WritebackTmp:   {:8} kB", self.writeback_tmp)?;
        writeln!(f, "CommitLimit:    {:8} kB", self.commit_limit)?;
        writeln!(f, "Committed_AS:   {:8} kB", self.committed_as)?;
        writeln!(f, "VmallocTotal:   {:8} kB", self.vmalloc_total)?;
        writeln!(f, "VmallocUsed:    {:8} kB", self.vmalloc_used)?;
        writeln!(f, "VmallocChunk:   {:8} kB", self.vmalloc_chunk)?;
        writeln!(f, "Percpu:         {:8} kB", self.percpu)?;
        writeln!(f, "AnonHugePages:  {:8} kB", self.anon_huge_pages)?;
        writeln!(f, "ShmemHugePages: {:8} kB", self.shmem_huge_pages)?;
        writeln!(f, "ShmemPmdMapped: {:8} kB", self.shmem_pmd_mapped)?;
        writeln!(f, "FileHugePages:  {:8} kB", self.file_huge_pages)?;
        writeln!(f, "FilePmdMapped:  {:8} kB", self.file_pmd_mapped)?;
        writeln!(f, "HugePages_Total:{:8}", self.huge_pages_total)?;
        writeln!(f, "HugePages_Free: {:8}", self.huge_pages_free)?;
        writeln!(f, "HugePages_Rsvd: {:8}", self.huge_pages_rsvd)?;
        writeln!(f, "HugePages_Surp: {:8}", self.huge_pages_surp)?;
        writeln!(f, "Hugepagesize:   {:8} kB", self.hugepagesize)?;
        writeln!(f, "Hugetlb:        {:8} kB", self.hugetlb)?;
        writeln!(f, "DirectMap4k:    {:8} kB", self.direct_map_4k)?;
        writeln!(f, "DirectMap2M:    {:8} kB", self.direct_map_2m)?;
        writeln!(f, "DirectMap1G:    {:8} kB", self.direct_map_1g)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ProcStat {
    pub cpu: Vec<ProcStatCpu>,
    pub paged: usize,
    pub paged_out: usize,
    pub swap_in: usize,
    pub swap_out: usize,
    pub intr: usize,
    pub ctxt: usize,
    pub btime: i64,
    pub processes: u64,
    pub procs_running: u32,
    pub procs_blocked: u32,
}
impl Display for ProcStat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for cpu in self.cpu.iter() {
            cpu.fmt(f)?;
        }
        writeln!(f, "page {} {}", self.paged, self.paged_out)?;
        writeln!(f, "swap {} {}", self.swap_in, self.swap_out)?;
        writeln!(f, "intr {}", self.intr)?;
        writeln!(f, "ctxt {}", self.ctxt)?;
        writeln!(f, "btime {}", self.btime)?;
        writeln!(f, "processes {}", self.processes)?;
        writeln!(f, "procs_running {}", self.procs_running)?;
        writeln!(f, "procs_blocked {}", self.procs_blocked)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ProcStatCpu {
    pub cpu_id: Option<usize>,
    pub user: u32,
    pub nice: u32,
    pub system: u32,
    pub idle: u32,
    pub iowait: u32,
    pub irq: u32,
    pub softirq: u32,
    pub steal: u32,
    pub guest: u32,
    pub guest_nice: u32,
}
impl Display for ProcStatCpu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let cpu_id = self.cpu_id.map(|x| x.to_string()).unwrap_or_default();
        writeln!(
            f,
            "cpu{} {} {} {} {} {} {} {} {} {} {}",
            cpu_id,
            self.user,
            self.nice,
            self.system,
            self.idle,
            self.iowait,
            self.irq,
            self.softirq,
            self.steal,
            self.guest,
            self.guest_nice
        )
    }
}

#[derive(Debug, Clone)]
pub struct ProcLoadavg {
    pub loadavg: [f64; 3],
    pub proc_running: u32,
    pub proc_total: u32,
    pub last_pid_running: i32,
}
impl Display for ProcLoadavg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "{} {} {} {}/{} {}",
            self.loadavg[0],
            self.loadavg[1],
            self.loadavg[2],
            self.proc_running,
            self.proc_total,
            self.last_pid_running
        )
    }
}

#[derive(Debug, Clone)]
pub struct ProcCpuinfo<T>(pub Vec<T>);
impl<T: Display> Display for ProcCpuinfo<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for i in self.0.iter() {
            writeln!(f, "{i}")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct X86ProcCpuinfoEntry {
    pub processor: usize,
    pub vendor_id: String,
    pub cpu_family: u32,
    pub model: u32,
    pub model_name: String,
    pub stepping: u32,
    pub microcode: u32,
    pub cpu_mhz: f64,
    pub cache_size_kb: usize,
    pub physical_id: usize,
    pub siblings: usize,
    pub core_id: usize,
    pub cpu_cores: usize,
    pub apicid: usize,
    pub initial_apicid: usize,
    pub fpu: bool,
    pub fpu_exception: bool,
    pub cpuid_level: u32,
    pub wp: bool,
    pub flags: Vec<&'static str>,
    pub vmx_flags: Vec<&'static str>,
    pub bugs: Vec<&'static str>,
    pub bogomips: f64,
    pub cflush_size: u64,
    pub cache_alignment: u64,
    pub address_sizes: (u8, u8),
    pub power_management: String,
}
impl Display for X86ProcCpuinfoEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let bool_yesno = |x| if x { "yes" } else { "no" };
        writeln!(f, "processor: {}", self.processor)?;
        writeln!(f, "vendor_id: {}", self.vendor_id)?;
        writeln!(f, "cpu family: {}", self.cpu_family)?;
        writeln!(f, "model: {}", self.model)?;
        writeln!(f, "model name: {}", self.model_name)?;
        writeln!(f, "stepping: {}", self.stepping)?;
        writeln!(f, "microcode: 0x{:x}", self.microcode)?;
        writeln!(f, "cpu MHz: {}", self.cpu_mhz)?;
        writeln!(f, "cache size: {} kB", self.cache_size_kb)?;
        writeln!(f, "physical id: {}", self.physical_id)?;
        writeln!(f, "siblings: {}", self.siblings)?;
        writeln!(f, "core id: {}", self.core_id)?;
        writeln!(f, "cpu cores: {}", self.cpu_cores)?;
        writeln!(f, "apicid: {}", self.apicid)?;
        writeln!(f, "initial apicid: {}", self.initial_apicid)?;
        writeln!(f, "fpu: {}", bool_yesno(self.fpu))?;
        writeln!(f, "fpu_exception: {}", bool_yesno(self.fpu_exception))?;
        writeln!(f, "cpuid level: {}", self.cpuid_level)?;
        writeln!(f, "wp: {}", bool_yesno(self.wp))?;
        write!(f, "flags: ")?;
        fmt_vec_space_split(f, &self.flags)?;
        write!(f, "vmx flags: ")?;
        fmt_vec_space_split(f, &self.vmx_flags)?;
        write!(f, "bugs: ")?;
        fmt_vec_space_split(f, &self.bugs)?;
        writeln!(f, "bogomips: {}", self.bogomips)?;
        writeln!(f, "cflush size: {}", self.cflush_size)?;
        writeln!(f, "cache_alignment: {}", self.cache_alignment)?;
        writeln!(
            f,
            "address sizes: {} bits physical, {} bits virtual",
            self.address_sizes.0, self.address_sizes.1
        )?;
        writeln!(f, "power management: {}", self.power_management)?;
        Ok(())
    }
}

/// An error while parsing a data structure.
#[derive(Debug, Clone)]
pub struct ParseError;
impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Failed to parse data structure")
    }
}
impl std::error::Error for ParseError {}

fn fmt_vec_space_split<T: Display>(f: &mut std::fmt::Formatter<'_>, v: &[T]) -> std::fmt::Result {
    for i in v {
        write!(f, "{i} ")?;
    }
    writeln!(f)?;
    Ok(())
}
