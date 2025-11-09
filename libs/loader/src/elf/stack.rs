use super::auxv::AuxiliaryInfo;
use std::alloc::Layout;

/// Jumps to a program entry with given information about the initial stack.
#[cfg(target_arch = "x86_64")]
pub fn jump(entry: *const u8, args: &[&[u8]], envs: &[&[u8]], auxv: AuxiliaryInfo) -> ! {
    unsafe {
        let stack_info = StackInfo::new(args, envs, auxv);
        let stkinfo_ptr = stack_info.0.as_ptr();
        let stkinfo_len = stack_info.0.len() * size_of::<usize>();
        rtenv::emuctx::enter_emulated();

        core::arch::asm!(
            "sub rsp, {stkinfo_len}",
            "mov rdi, rsp",
            "mov rsi, {stkinfo_ptr}",
            "mov rcx, {stkinfo_len}",
            "rep movsb",
            "jmp {entry}",

            stkinfo_ptr = in(reg) stkinfo_ptr,
            stkinfo_len = in(reg) stkinfo_len,
            entry = in(reg) entry,
            options(noreturn),
        );
    }
}

/// Stack information.
#[derive(Debug)]
pub struct StackInfo(Vec<usize>);
impl StackInfo {
    /// Builds a [`StackInfo`] instance with given information.
    pub fn new(args: &[&[u8]], envs: &[&[u8]], auxv: AuxiliaryInfo) -> Self {
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

        let mut vec = Vec::with_capacity(args.len() + envs.len() + 64);

        vec.push(args.len()); // push argc
        args.iter().for_each(|x| vec.push(allocate_string(x))); // push argv elements
        vec.push(0); // push argv terminator
        envs.iter().for_each(|x| vec.push(allocate_string(x))); // push envp elements
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
