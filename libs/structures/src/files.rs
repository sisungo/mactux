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

/// An error while parsing a data structure.
#[derive(Debug, Clone)]
pub struct ParseError;
impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Failed to parse data structure")
    }
}
impl std::error::Error for ParseError {}
