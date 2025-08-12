use crate::auxv::AuxiliaryInfo;
use std::alloc::Layout;

#[cfg(target_arch = "x86_64")]
pub fn jump<'a, 'b>(
    entry: *const u8,
    args: impl ExactSizeIterator<Item = &'a [u8]>,
    envs: impl Iterator<Item = &'a [u8]>,
    auxv: AuxiliaryInfo,
) {
    unsafe {
        let stack_info = StackInfo::new(args, envs, auxv);
        rtenv::emuctx::enter_emulated();

        core::arch::asm!(
            "sub rsp, {sil}",
            "mov rdi, rsp",
            "mov rsi, {sip}",
            "mov rcx, {sil}",
            "rep movsb",
            "jmp {entry}",

            sip = in(reg) stack_info.0.as_ptr(),
            sil = in(reg) stack_info.0.len() * size_of::<usize>(),
            entry = in(reg) entry,
        );
    }
}

#[derive(Debug)]
pub struct StackInfo(Vec<usize>);
impl StackInfo {
    pub fn new<'a, 'b>(
        args: impl ExactSizeIterator<Item = &'a [u8]>,
        envs: impl Iterator<Item = &'b [u8]>,
        auxv: AuxiliaryInfo,
    ) -> Self {
        fn allocate_string(s: &[u8]) -> usize {
            let len = s.len() + 1;
            let ptr = unsafe { std::alloc::alloc(Layout::array::<u8>(len).unwrap()) };
            if ptr.is_null() {
                panic!("Out of memory");
            }
            unsafe {
                std::ptr::copy_nonoverlapping(s.as_ptr(), ptr, s.len());
                *ptr.add(s.len()) = 0;
            }
            ptr as usize
        }

        let mut vec = Vec::with_capacity(args.len() + 96);

        vec.push(args.len()); // push argc
        args.for_each(|x| vec.push(allocate_string(x))); // push argv elements
        vec.push(0); // push argv terminator
        envs.for_each(|x| vec.push(allocate_string(x))); // push envp elements
        vec.push(0); // push envp terminator
        auxv.push_to_stack(&mut vec); // push auxv along with terminator

        if vec.len() % 2 != 0 {
            vec.push(0); // align to even number of elements
        }

        Self(vec)
    }
}
impl Drop for StackInfo {
    fn drop(&mut self) {
        let mut met_zeroes = 0;
        for &i in self.0.iter().skip(1) {
            // We only release argv and envp, not auxv.
            if met_zeroes >= 2 {
                break;
            }

            if i == 0 {
                met_zeroes += 1;
            } else {
                unsafe {
                    let len = libc::strlen(i as _);
                    std::alloc::dealloc(i as _, Layout::array::<u8>(len).unwrap());
                }
            }
        }
    }
}
