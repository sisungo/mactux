/// Information about an auxiliary vector.
#[derive(Debug)]
pub struct AuxiliaryInfo {
    pub exec_fd: usize,
    pub phdr_base: usize,
    pub phdr_size: usize,
    pub phdr_count: usize,
    pub entry: usize,
    pub base: usize,
    pub random: *const [u8; 64],
}
impl AuxiliaryInfo {
    /// Pushes all the information to a [`Vec<usize>`] stack, following the format specified in System V ABI.
    pub fn push_to_stack(&self, stack: &mut Vec<usize>) {
        // Push page size.
        stack.push(AuxType::PageSz as usize);
        stack.push(super::page_size());

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

        // Push the random bytes.
        stack.push(AuxType::Random as usize);
        stack.push(self.random as usize);

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

/// Type of an auxiliary vector entry.
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
    Random = 25,
    Sysinfo = 32,
    SysinfoEhdr = 33,
}
