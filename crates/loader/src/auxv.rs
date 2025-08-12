#[derive(Debug)]
pub struct AuxiliaryInfo {
    pub exec_fd: usize,
    pub phdr_base: usize,
    pub phdr_size: usize,
    pub phdr_count: usize,
    pub entry: usize,
    pub base: usize,
}
impl AuxiliaryInfo {
    pub fn push_to_stack(&self, stack: &mut Vec<usize>) {
        // The page size is fixed on macOS for each architecture.
        #[cfg(target_arch = "x86_64")]
        {
            stack.push(AuxType::PageSz as usize);
            stack.push(0x1000); // 4KB page size
        }
        #[cfg(target_arch = "aarch64")]
        {
            stack.push(AuxType::PageSz as usize);
            stack.push(0x4000); // 16KB page size
        }

        // Push PHDR information.
        stack.push(AuxType::Phdr as usize);
        stack.push(self.phdr_base);
        stack.push(AuxType::PhEnt as usize);
        stack.push(self.phdr_size);
        stack.push(AuxType::PhNum as usize);
        stack.push(self.phdr_count);
        stack.push(AuxType::Base as usize);
        stack.push(self.base);
        stack.push(AuxType::Entry as usize);
        stack.push(self.entry);

        // Push exec fd.
        stack.push(AuxType::ExecFd as usize);
        stack.push(self.exec_fd);

        // Push vDSO.
        stack.push(AuxType::Sysinfo as usize);
        stack.push(0);
        stack.push(AuxType::SysinfoEhdr as usize);
        stack.push(0);

        // Push the terminator.
        stack.push(AuxType::Null as usize);
        stack.push(0);
    }
}

#[derive(Debug, Clone, Copy)]
enum AuxType {
    Null = 0,
    ExecFd = 2,
    Phdr = 3,
    PhEnt = 4,
    PhNum = 5,
    PageSz = 6,
    Base = 7,
    Entry = 9,
    Sysinfo = 32,
    SysinfoEhdr = 33,
}
