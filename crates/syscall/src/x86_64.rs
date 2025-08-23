use crate::{SystemCallHandler, UcontextExt, common::*};
use libc::{c_int, siginfo_t, ucontext_t};
use macros::syscall;
use structures::{
    error::LxError,
    process::{CloneArgs, CloneFlags},
};

/// Handler of `SIGSYS` signal.
pub unsafe extern "C" fn handle_sigsys(_: c_int, info: &siginfo_t, uap: &mut ucontext_t) {
    if rtenv::signal::is_async(info) {
        rtenv::error_report::fast_fail();
    }

    unsafe {
        perform(uap);
    }
}

impl UcontextExt for libc::ucontext_t {
    fn sysno(&self) -> usize {
        thread_state(self).__rax as _
    }

    fn arg0(&self) -> usize {
        thread_state(self).__rdi as _
    }

    fn arg1(&self) -> usize {
        thread_state(self).__rsi as _
    }

    fn arg2(&self) -> usize {
        thread_state(self).__rdx as _
    }

    fn arg3(&self) -> usize {
        thread_state(self).__r10 as _
    }

    fn arg4(&self) -> usize {
        thread_state(self).__r8 as _
    }

    fn arg5(&self) -> usize {
        thread_state(self).__r9 as _
    }

    fn ret(&mut self, value: usize) {
        thread_state_mut(self).__rax = value as u64;
    }
}

fn thread_state(x: &libc::ucontext_t) -> &libc::__darwin_x86_thread_state64 {
    unsafe { &(*x.uc_mcontext).__ss }
}

fn thread_state_mut(x: &mut libc::ucontext_t) -> &mut libc::__darwin_x86_thread_state64 {
    unsafe { &mut (*x.uc_mcontext).__ss }
}

/// Performs a system call.
unsafe fn perform(uctx: &mut libc::ucontext_t) {
    unsafe {
        let handler = SYSTEM_CALL_HANDLERS
            .get(uctx.sysno() as usize)
            .copied()
            .unwrap_or(sys_invalid);
        handler(uctx);
    }
}

const SYSTEM_CALL_HANDLERS: &[SystemCallHandler] = &[
    sys_read,              // 0
    sys_write,             // 1
    sys_open,              // 2
    sys_close,             // 3
    sys_stat,              // 4
    sys_fstat,             // 5
    sys_lstat,             // 6
    sys_poll,              // 7
    sys_lseek,             // 8
    sys_mmap,              // 9
    sys_mprotect,          // 10
    sys_munmap,            // 11
    sys_brk,               // 12
    sys_rt_sigaction,      // 13
    sys_rt_sigprocmask,    // 14
    sys_rt_sigreturn,      // 15
    sys_ioctl,             // 16
    sys_pread64,           // 17
    sys_pwrite64,          // 18
    sys_readv,             // 19
    sys_writev,            // 20
    sys_access,            // 21
    sys_pipe,              // 22
    sys_select,            // 23
    sys_sched_yield,       // 24
    sys_mremap,            // 25
    sys_msync,             // 26
    sys_mincore,           // 27
    sys_madvise,           // 28
    sys_invalid,           // 29
    sys_invalid,           // 30
    sys_invalid,           // 31
    sys_dup,               // 32
    sys_dup2,              // 33
    sys_pause,             // 34
    sys_nanosleep,         // 35
    sys_invalid,           // 36
    sys_alarm,             // 37
    sys_invalid,           // 38
    sys_getpid,            // 39
    sys_sendfile,          // 40
    sys_socket,            // 41
    sys_connect,           // 42
    sys_accept,            // 43
    sys_invalid,           // 44
    sys_invalid,           // 45
    sys_invalid,           // 46
    sys_invalid,           // 47
    sys_shutdown,          // 48
    sys_bind,              // 49
    sys_listen,            // 50
    sys_getsockname,       // 51
    sys_getpeername,       // 52
    sys_invalid,           // 53
    sys_setsockopt,        // 54
    sys_getsockopt,        // 55
    sys_clone,             // 56
    sys_fork,              // 57
    sys_vfork,             // 58
    sys_execve,            // 59
    sys_exit,              // 60
    sys_wait4,             // 61
    sys_kill,              // 62
    sys_uname,             // 63
    sys_invalid,           // 64
    sys_invalid,           // 65
    sys_invalid,           // 66
    sys_invalid,           // 67
    sys_invalid,           // 68
    sys_invalid,           // 69
    sys_invalid,           // 70
    sys_invalid,           // 71
    sys_fcntl,             // 72
    sys_flock,             // 73
    sys_fsync,             // 74
    sys_fdatasync,         // 75
    sys_truncate,          // 76
    sys_ftruncate,         // 77
    sys_invalid,           // 78
    sys_getcwd,            // 79
    sys_chdir,             // 80
    sys_fchdir,            // 81
    sys_rename,            // 82
    sys_mkdir,             // 83
    sys_rmdir,             // 84
    sys_invalid,           // 85
    sys_invalid,           // 86
    sys_unlink,            // 87
    sys_symlink,           // 88
    sys_readlink,          // 89
    sys_invalid,           // 90
    sys_invalid,           // 91
    sys_chown,             // 92
    sys_fchown,            // 93
    sys_invalid,           // 94
    sys_umask,             // 95
    sys_gettimeofday,      // 96
    sys_invalid,           // 97
    sys_getrusage,         // 98
    sys_sysinfo,           // 99
    sys_invalid,           // 100
    sys_invalid,           // 101
    sys_getuid,            // 102
    sys_invalid,           // 103
    sys_getgid,            // 104
    sys_setuid,            // 105
    sys_setgid,            // 106
    sys_geteuid,           // 107
    sys_getegid,           // 108
    sys_setpgid,           // 109
    sys_getppid,           // 110
    sys_getpgrp,           // 111
    sys_invalid,           // 112
    sys_invalid,           // 113
    sys_invalid,           // 114
    sys_getgroups,         // 115
    sys_invalid,           // 116
    sys_invalid,           // 117
    sys_invalid,           // 118
    sys_invalid,           // 119
    sys_invalid,           // 120
    sys_getpgid,           // 121
    sys_invalid,           // 122
    sys_invalid,           // 123
    sys_invalid,           // 124
    sys_invalid,           // 125
    sys_invalid,           // 126
    sys_invalid,           // 127
    sys_invalid,           // 128
    sys_invalid,           // 129
    sys_invalid,           // 130
    sys_invalid,           // 131
    sys_invalid,           // 132
    sys_invalid,           // 133
    sys_uselib,            // 134
    sys_invalid,           // 135
    sys_invalid,           // 136
    sys_invalid,           // 137
    sys_invalid,           // 138
    sys_sysfs,             // 139
    sys_invalid,           // 140
    sys_invalid,           // 141
    sys_invalid,           // 142
    sys_invalid,           // 143
    sys_invalid,           // 144
    sys_invalid,           // 145
    sys_invalid,           // 146
    sys_invalid,           // 147
    sys_invalid,           // 148
    sys_invalid,           // 149
    sys_invalid,           // 150
    sys_invalid,           // 151
    sys_invalid,           // 152
    sys_invalid,           // 153
    sys_invalid,           // 154
    sys_invalid,           // 155
    sys_invalid,           // 156
    sys_prctl,             // 157
    sys_arch_prctl,        // 158
    sys_invalid,           // 159
    sys_invalid,           // 160
    sys_invalid,           // 161
    sys_sync,              // 162
    sys_acct,              // 163
    sys_invalid,           // 164
    sys_invalid,           // 165
    sys_invalid,           // 166
    sys_invalid,           // 167
    sys_invalid,           // 168
    sys_invalid,           // 169
    sys_sethostname,       // 170
    sys_setdomainname,     // 171
    sys_invalid,           // 172
    sys_invalid,           // 173
    sys_invalid,           // 174
    sys_invalid,           // 175
    sys_invalid,           // 176
    sys_invalid,           // 177
    sys_invalid,           // 178
    sys_invalid,           // 179
    sys_invalid,           // 180
    sys_invalid,           // 181
    sys_invalid,           // 182
    sys_invalid,           // 183
    sys_invalid,           // 184
    sys_invalid,           // 185
    sys_gettid,            // 186
    sys_invalid,           // 187
    sys_invalid,           // 188
    sys_invalid,           // 189
    sys_invalid,           // 190
    sys_invalid,           // 191
    sys_invalid,           // 192
    sys_invalid,           // 193
    sys_listxattr,         // 194
    sys_llistxattr,        // 195
    sys_flistxattr,        // 196
    sys_invalid,           // 197
    sys_invalid,           // 198
    sys_invalid,           // 199
    sys_tkill,             // 200
    sys_time,              // 201
    sys_futex,             // 202
    sys_sched_setaffinity, // 203
    sys_sched_getaffinity, // 204
    sys_invalid,           // 205
    sys_invalid,           // 206
    sys_invalid,           // 207
    sys_invalid,           // 208
    sys_invalid,           // 209
    sys_invalid,           // 210
    sys_invalid,           // 211
    sys_invalid,           // 212
    sys_invalid,           // 213
    sys_invalid,           // 214
    sys_invalid,           // 215
    sys_invalid,           // 216
    sys_getdents64,        // 217
    sys_set_tid_address,   // 218
    sys_invalid,           // 219
    sys_invalid,           // 220
    sys_fadvise64,         // 221
    sys_invalid,           // 222
    sys_invalid,           // 223
    sys_invalid,           // 224
    sys_invalid,           // 225
    sys_invalid,           // 226
    sys_invalid,           // 227
    sys_clock_gettime,     // 228
    sys_invalid,           // 229
    sys_invalid,           // 230
    sys_exit_group,        // 231
    sys_invalid,           // 232
    sys_invalid,           // 233
    sys_invalid,           // 234
    sys_invalid,           // 235
    sys_invalid,           // 236
    sys_invalid,           // 237
    sys_invalid,           // 238
    sys_invalid,           // 239
    sys_invalid,           // 240
    sys_invalid,           // 241
    sys_invalid,           // 242
    sys_invalid,           // 243
    sys_invalid,           // 244
    sys_invalid,           // 245
    sys_invalid,           // 246
    sys_invalid,           // 247
    sys_invalid,           // 248
    sys_invalid,           // 249
    sys_invalid,           // 250
    sys_invalid,           // 251
    sys_invalid,           // 252
    sys_invalid,           // 253
    sys_invalid,           // 254
    sys_invalid,           // 255
    sys_invalid,           // 256
    sys_openat,            // 257
    sys_invalid,           // 258
    sys_invalid,           // 259
    sys_invalid,           // 260
    sys_invalid,           // 261
    sys_newfstatat,        // 262
    sys_invalid,           // 263
    sys_invalid,           // 264
    sys_invalid,           // 265
    sys_invalid,           // 266
    sys_invalid,           // 267
    sys_invalid,           // 268
    sys_invalid,           // 269
    sys_pselect6,          // 270
    sys_ppoll,             // 271
    sys_invalid,           // 272
    sys_set_robust_list,   // 273
    sys_invalid,           // 274
    sys_invalid,           // 275
    sys_invalid,           // 276
    sys_invalid,           // 277
    sys_invalid,           // 278
    sys_invalid,           // 279
    sys_invalid,           // 280
    sys_invalid,           // 281
    sys_invalid,           // 282
    sys_invalid,           // 283
    sys_eventfd,           // 284
    sys_invalid,           // 285
    sys_invalid,           // 286
    sys_invalid,           // 287
    sys_accept4,           // 288
    sys_invalid,           // 289
    sys_eventfd2,          // 290
    sys_invalid,           // 291
    sys_invalid,           // 292
    sys_pipe2,             // 293
    sys_invalid,           // 294
    sys_invalid,           // 295
    sys_invalid,           // 296
    sys_invalid,           // 297
    sys_invalid,           // 298
    sys_invalid,           // 299
    sys_invalid,           // 300
    sys_invalid,           // 301
    sys_prlimit64,         // 302
    sys_invalid,           // 303
    sys_invalid,           // 304
    sys_invalid,           // 305
    sys_syncfs,            // 306
    sys_invalid,           // 307
    sys_invalid,           // 308
    sys_invalid,           // 309
    sys_invalid,           // 310
    sys_invalid,           // 311
    sys_invalid,           // 312
    sys_invalid,           // 313
    sys_invalid,           // 314
    sys_invalid,           // 315
    sys_invalid,           // 316
    sys_invalid,           // 317
    sys_getrandom,         // 318
    sys_invalid,           // 319
    sys_invalid,           // 320
    sys_invalid,           // 321
    sys_invalid,           // 322
    sys_invalid,           // 323
    sys_invalid,           // 324
    sys_invalid,           // 325
    sys_copy_file_range,   // 326
    sys_invalid,           // 327
    sys_invalid,           // 328
    sys_invalid,           // 329
    sys_invalid,           // 330
    sys_invalid,           // 331
    sys_statx,             // 332
    sys_invalid,           // 333
    sys_rseq,              // 334
    sys_invalid,           // 335
    sys_invalid,           // 336
    sys_invalid,           // 337
    sys_invalid,           // 338
    sys_invalid,           // 339
    sys_invalid,           // 340
    sys_invalid,           // 341
    sys_invalid,           // 342
    sys_invalid,           // 343
    sys_invalid,           // 344
    sys_invalid,           // 345
    sys_invalid,           // 346
    sys_invalid,           // 347
    sys_invalid,           // 348
    sys_invalid,           // 349
    sys_invalid,           // 350
    sys_invalid,           // 351
    sys_invalid,           // 352
    sys_invalid,           // 353
    sys_invalid,           // 354
    sys_invalid,           // 355
    sys_invalid,           // 356
    sys_invalid,           // 357
    sys_invalid,           // 358
    sys_invalid,           // 359
    sys_invalid,           // 360
    sys_invalid,           // 361
    sys_invalid,           // 362
    sys_invalid,           // 363
    sys_invalid,           // 364
    sys_invalid,           // 365
    sys_invalid,           // 366
    sys_invalid,           // 367
    sys_invalid,           // 368
    sys_invalid,           // 369
    sys_invalid,           // 370
    sys_invalid,           // 371
    sys_invalid,           // 372
    sys_invalid,           // 373
    sys_invalid,           // 374
    sys_invalid,           // 375
    sys_invalid,           // 376
    sys_invalid,           // 377
    sys_invalid,           // 378
    sys_invalid,           // 379
    sys_invalid,           // 380
    sys_invalid,           // 381
    sys_invalid,           // 382
    sys_invalid,           // 383
    sys_invalid,           // 384
    sys_invalid,           // 385
    sys_invalid,           // 386
    sys_invalid,           // 387
    sys_invalid,           // 388
    sys_invalid,           // 389
    sys_invalid,           // 390
    sys_invalid,           // 391
    sys_invalid,           // 392
    sys_invalid,           // 393
    sys_invalid,           // 394
    sys_invalid,           // 395
    sys_invalid,           // 396
    sys_invalid,           // 397
    sys_invalid,           // 398
    sys_invalid,           // 399
    sys_invalid,           // 400
    sys_invalid,           // 401
    sys_invalid,           // 402
    sys_invalid,           // 403
    sys_invalid,           // 404
    sys_invalid,           // 405
    sys_invalid,           // 406
    sys_invalid,           // 407
    sys_invalid,           // 408
    sys_invalid,           // 409
    sys_invalid,           // 410
    sys_invalid,           // 411
    sys_invalid,           // 412
    sys_invalid,           // 413
    sys_invalid,           // 414
    sys_invalid,           // 415
    sys_invalid,           // 416
    sys_invalid,           // 417
    sys_invalid,           // 418
    sys_invalid,           // 419
    sys_invalid,           // 420
    sys_invalid,           // 421
    sys_invalid,           // 422
    sys_invalid,           // 423
    sys_invalid,           // 424
    sys_invalid,           // 425
    sys_invalid,           // 426
    sys_invalid,           // 427
    sys_invalid,           // 428
    sys_invalid,           // 429
    sys_invalid,           // 430
    sys_invalid,           // 431
    sys_invalid,           // 432
    sys_invalid,           // 433
    sys_invalid,           // 434
    sys_invalid,           // 435
    sys_invalid,           // 436
    sys_invalid,           // 437
    sys_invalid,           // 438
    sys_faccessat2,        // 439
    sys_invalid,           // 440
    sys_invalid,           // 441
    sys_invalid,           // 442
    sys_invalid,           // 443
    sys_invalid,           // 444
    sys_invalid,           // 445
    sys_invalid,           // 446
    sys_invalid,           // 447
    sys_invalid,           // 448
    sys_invalid,           // 449
    sys_invalid,           // 450
    sys_invalid,           // 451
    sys_invalid,           // 452
    sys_invalid,           // 453
    sys_invalid,           // 454
    sys_invalid,           // 455
    sys_invalid,           // 456
    sys_invalid,           // 457
    sys_invalid,           // 458
    sys_invalid,           // 459
    sys_invalid,           // 460
    sys_invalid,           // 461
    sys_invalid,           // 462
    sys_invalid,           // 463
    sys_invalid,           // 464
    sys_invalid,           // 465
    sys_invalid,           // 466
    sys_invalid,           // 467
    sys_invalid,           // 468
    sys_invalid,           // 469
    sys_invalid,           // 470
    sys_invalid,           // 471
    sys_invalid,           // 472
    sys_invalid,           // 473
    sys_invalid,           // 474
    sys_invalid,           // 475
    sys_invalid,           // 476
    sys_invalid,           // 477
    sys_invalid,           // 478
    pseudo_restorectx,     // 479
];

#[syscall]
unsafe fn sys_arch_prctl(op: usize, arg: usize) -> Result<(), LxError> {
    const ARCH_SET_FS: usize = 0x1002;
    const ARCH_SET_GS: usize = 0x1001;

    match op {
        ARCH_SET_FS => {
            rtenv::emuctx::x86_64_set_emulated_gsbase(arg as _);
            Ok(())
        }
        ARCH_SET_GS => Err(LxError::EOPNOTSUPP),
        _ => Err(LxError::EINVAL),
    }
}

unsafe fn sys_rt_sigreturn(ctx: &mut libc::ucontext_t) {
    unsafe {
        rtenv::signal::sigreturn(ctx);
    }
}

/// Convenient macro to implement indirect system calls on x86_64.
///
/// Different from normal system calls, indirect system calls do not execute their effective code in the signal handler. They
/// return from the signal handler, jumping the context to their effective code, and when it completes, it calls the
/// `pseudo_restorectx` pseudo system call to restore the original user context.
///
/// It's slower, but due to macOS limitations, some operations fail in the signal handler context. For example, if we call
/// `fork()` in the signal handler, the child immediately dies with `SIGTRAP` after returning from the handler.
macro_rules! impl_syscall_indirect {
    ($name:ident = $blk:expr) => {
        unsafe fn $name(uctx: &mut libc::ucontext_t) {
            unsafe extern "sysv64" fn __impl(mut ctx: Box<libc::__darwin_mcontext64>) -> ! {
                unsafe {
                    rtenv::emuctx::leave_emulated();
                    ctx.__ss.__rax = $blk(&mut *ctx);
                    rtenv::emuctx::enter_emulated();
                    core::arch::asm!(
                        "mov rdi, {}",
                        "mov rax, 479", // pseudo_restorectx
                        "syscall",
                        in(reg) Box::into_raw(ctx),
                        options(nostack, noreturn),
                    );
                }
            }

            unsafe {
                rtenv::emuctx::leave_emulated();
                let original_ctx = Box::new(*uctx.uc_mcontext);
                (*uctx.uc_mcontext).__ss.__rdi = Box::into_raw(original_ctx) as usize as u64;
                (*uctx.uc_mcontext).__ss.__rip = __impl as usize as u64;
                rtenv::emuctx::enter_emulated();
            }
        }
    };
}

impl_syscall_indirect!(
    sys_clone = |mctx: &mut libc::__darwin_mcontext64| {
        let flags = CloneFlags::from_bits_retain(mctx.__ss.__rdi as _);
        let stack = mctx.__ss.__rsi;
        let parent_tid = mctx.__ss.__rdx;
        let child_tid = mctx.__ss.__r10;
        let tls = mctx.__ss.__r8;
        let clone_args = CloneArgs {
            flags: flags.bits() as _,
            pidfd: 0,
            child_tid,
            parent_tid,
            exit_signal: 0,
            stack,
            stack_size: 0,
            tls,
            set_tid: 0,
            set_tid_size: 0,
            cgroup: 0,
        };
        match rtenv::process::clone(clone_args) {
            Ok(0) => {
                if stack != 0 {
                    mctx.__ss.__rsp = stack;
                }
                0
            }
            Ok(n) => n as _,
            Err(err) => -(err.0 as i32) as u64,
        }
    }
);
impl_syscall_indirect!(
    sys_fork = |_| {
        match rtenv::process::fork() {
            Ok(n) => n as _,
            Err(err) => -(err.0 as i32) as u64,
        }
    }
);
use sys_fork as sys_vfork;

/// The `restorectx` pseudo system call \(479\).
///
/// This is only used to implement indirect system calls. See [`impl_syscall_indirect`] for details.
unsafe fn pseudo_restorectx(uctx: &mut libc::ucontext_t) {
    unsafe {
        rtenv::emuctx::leave_emulated();
        let ctx = Box::from_raw(uctx.arg0() as *mut libc::__darwin_mcontext64);
        *uctx.uc_mcontext = *ctx;
        rtenv::emuctx::enter_emulated();
    }
}
