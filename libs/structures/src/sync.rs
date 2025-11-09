use crate::error::LxError;
use bitflags::bitflags;
use libc::c_int;
use std::ptr::NonNull;

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct FutexOp(pub c_int);
impl FutexOp {
    #[inline]
    pub const fn opts(self) -> FutexOpts {
        FutexOpts::from_bits_retain(self.0 & !0x7f)
    }

    #[inline]
    pub const fn cmd(self) -> FutexCmd {
        FutexCmd(self.0 & 0x7f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct FutexCmd(pub c_int);
impl FutexCmd {
    pub const FUTEX_WAIT: FutexCmd = FutexCmd(0);
    pub const FUTEX_WAKE: FutexCmd = FutexCmd(1);
    pub const FUTEX_FD: FutexCmd = FutexCmd(2);
    pub const FUTEX_REQUEUE: FutexCmd = FutexCmd(3);
    pub const FUTEX_CMP_REQUEUE: FutexCmd = FutexCmd(4);
    pub const FUTEX_WAKE_OP: FutexCmd = FutexCmd(5);
    pub const FUTEX_LOCK_PI: FutexCmd = FutexCmd(6);
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct FutexOpts: c_int {
        const FUTEX_PRIVATE_FLAGS = 128;
        const FUTEX_CLOCK_REALTIME = 256;
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct FutexWakeOpVal3(pub u32);
impl FutexWakeOpVal3 {
    pub const fn op(self) -> FutexWakeOp {
        FutexWakeOp(self.0 >> 28)
    }

    pub const fn cmp(self) -> FutexWakeOpCmp {
        FutexWakeOpCmp((self.0 >> 24) & 0xff)
    }

    pub const fn oparg(self) -> u32 {
        (self.0 >> 12) & 0xfff
    }

    pub const fn cmparg(self) -> u32 {
        self.0 & 0xfff
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct FutexWakeOp(u32);
impl FutexWakeOp {
    pub const FUTEX_OP_SET: Self = Self(0);
    pub const FUTEX_OP_ADD: Self = Self(1);
    pub const FUTEX_OP_OR: Self = Self(2);
    pub const FUTEX_OP_ANDN: Self = Self(3);
    pub const FUTEX_OP_XOR: Self = Self(4);

    pub const fn perform(self, oldval: &mut u32, oparg: u32) -> Result<(), LxError> {
        match self {
            Self::FUTEX_OP_SET => *oldval = oparg,
            Self::FUTEX_OP_ADD => *oldval += oparg,
            Self::FUTEX_OP_OR => *oldval |= oparg,
            Self::FUTEX_OP_ANDN => *oldval &= !oparg,
            Self::FUTEX_OP_XOR => *oldval ^= oparg,
            _ => return Err(LxError::EINVAL),
        };

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct FutexWakeOpCmp(u32);
impl FutexWakeOpCmp {
    pub const FUTEX_OP_CMP_EQ: Self = Self(0);
    pub const FUTEX_OP_CMP_NE: Self = Self(1);
    pub const FUTEX_OP_CMP_LT: Self = Self(2);
    pub const FUTEX_OP_CMP_LE: Self = Self(3);
    pub const FUTEX_OP_CMP_GT: Self = Self(4);
    pub const FUTEX_OP_CMP_GE: Self = Self(5);

    pub const fn perform(self, oldval: u32, cmparg: u32) -> Result<bool, LxError> {
        Ok(match self {
            Self::FUTEX_OP_CMP_EQ => oldval == cmparg,
            Self::FUTEX_OP_CMP_NE => oldval != cmparg,
            Self::FUTEX_OP_CMP_LT => oldval < cmparg,
            Self::FUTEX_OP_CMP_LE => oldval <= cmparg,
            Self::FUTEX_OP_CMP_GT => oldval > cmparg,
            Self::FUTEX_OP_CMP_GE => oldval >= cmparg,
            _ => return Err(LxError::EINVAL),
        })
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct RobustList {
    pub next: Option<NonNull<Self>>,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct RobustListHead {
    pub list: RobustList,
    pub futex_offset: i64,
    pub list_op_pending: Option<NonNull<RobustList>>,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct RSeq {
    pub cpu_id_start: u32,
    pub cpu_id: u32,
    pub rseq_cs: *mut RSeqCs,
    pub flags: u32,
    pub node_id: u32,
    pub mm_cid: u32,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct RSeqCs {
    pub version: u32,
    pub flags: u32,
    pub start_ip: u64,
    pub post_commit_offset: u64,
    pub abort_ip: u64,
}
