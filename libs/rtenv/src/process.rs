use crate::{
    fs::FilesystemContext,
    ipc_client::{Client, with_client},
    posix_num, process,
    thread::{ThreadPubCtxMap, may_fork},
    util::posix_result,
};
use arc_swap::ArcSwap;
use rustc_hash::FxBuildHasher;
use std::{
    convert::Infallible,
    ffi::{OsString, c_int},
    mem::MaybeUninit,
    os::{fd::AsRawFd, unix::process::CommandExt},
    path::PathBuf,
    sync::Arc,
};
use structures::{
    ToApple,
    error::LxError,
    fs::{AT_FDCWD, AtFlags, FileMode, FileType, OpenFlags, StatxMask},
    internal::mactux_ipc::Request,
    mapper::with_pid_mapper,
    process::{ChildType, CloneArgs, CloneFlags},
    signal::{SigAction, SigNum},
    thread::is_tid,
};

static mut PROCESS_CTX: MaybeUninit<ProcessCtx> = MaybeUninit::uninit();

/// Context of a process.
#[derive(Debug)]
pub struct ProcessCtx {
    pub fs: FilesystemContext,
    pub thread_pubctx_map: ThreadPubCtxMap,
    pub sigactions: [ArcSwap<SigAction>; SigNum::_NSIG as usize],
    pub vfd_table: papaya::HashMap<c_int, u64, FxBuildHasher>,
    pub server_sock_path: ArcSwap<PathBuf>,
    pub important_fds: papaya::HashSet<c_int, FxBuildHasher>,
}

/// Installs the process context.
pub unsafe fn install() -> std::io::Result<()> {
    let fs = FilesystemContext::new();
    let thread_pubctx_map = ThreadPubCtxMap::new();
    let sigactions = std::array::from_fn(|_| ArcSwap::from(Arc::new(SigAction::new())));
    let vfd_table = papaya::HashMap::with_capacity_and_hasher(128, FxBuildHasher);
    let server_sock_path = ArcSwap::from(Arc::new(
        std::env::home_dir()
            .expect("cannot find home directory")
            .join(".mactux/mactux.sock"),
    ));
    unsafe {
        (*&raw mut PROCESS_CTX).as_mut_ptr().write(ProcessCtx {
            fs,
            thread_pubctx_map,
            sigactions,
            vfd_table,
            server_sock_path,
            important_fds: papaya::HashSet::default(),
        });
    }
    Ok(())
}

/// Returns a reference to current process context.
pub fn context() -> &'static ProcessCtx {
    unsafe { (*&raw const PROCESS_CTX).assume_init_ref() }
}

pub fn pid() -> i32 {
    let native_pid = unsafe { libc::getpid() };
    with_pid_mapper(|x| x.apple_to_linux(native_pid)).unwrap_or(native_pid)
}

pub fn ppid() -> i32 {
    let native_ppid = unsafe { libc::getppid() };
    with_pid_mapper(|x| x.apple_to_linux(native_ppid)).unwrap_or(native_ppid)
}

pub fn pgid(pid: i32) -> Result<i32, LxError> {
    let native_pid = with_pid_mapper(|x| x.linux_to_apple(pid))?;
    let native_pgid = unsafe { posix_num!(libc::getpgid(native_pid))? };
    with_pid_mapper(|x| x.apple_to_linux(native_pgid))
}

pub fn setpgid(pid: i32, pgid: i32) -> Result<(), LxError> {
    // TODO: support pid namespaces.
    unsafe { posix_result(libc::setpgid(pid, pgid)) }
}

pub unsafe fn exec(
    path: &[u8],
    argv: &[*const u8],
    envp: &[*const u8],
) -> Result<Infallible, LxError> {
    let fd = crate::fs::openat(
        AT_FDCWD,
        path.to_vec(),
        OpenFlags::O_CLOEXEC | OpenFlags::O_PATH,
        AtFlags::empty(),
        FileMode(0),
    )?;
    let stat =
        crate::fs::fstat(fd, StatxMask::STATX_MODE).inspect_err(|_| _ = crate::io::close(fd))?;
    _ = crate::io::close(fd);
    match stat.stx_mode.file_type() {
        FileType::Directory => return Err(LxError::EISDIR),
        _ => (),
    }
    if stat.stx_mode.permbits() & 0o111 == 0 {
        return Err(LxError::EPERM);
    }

    let mut args = Vec::with_capacity(argv.len() + 2 * envp.len() + 8);
    let mut argv = unsafe {
        argv.iter()
            .map(|&x| std::slice::from_raw_parts(x, libc::strlen(x as _)))
    };
    let envp = unsafe {
        envp.iter()
            .map(|&x| std::slice::from_raw_parts(x, libc::strlen(x as _)))
    };

    let mactux_exec = std::fs::canonicalize(std::env::current_exe().map_err(LxError::from)?)
        .map_err(LxError::from)?;

    let init_sock_fd = with_client(|client| {
        client.disable_cloexec().unwrap();
        client.as_raw_fd()
    });
    args.push(String::from("--init-sock-fd").into_bytes());
    args.push(init_sock_fd.to_string().into_bytes());

    // Pass current working directory.
    // If current value is "invalid", a '?' string just makes a false initialization, which inherits the "invalid" state.
    args.push(String::from("--cwd").into_bytes());
    args.push(crate::fs::getcwd());

    args.push(String::from("--server-sock-path").into_bytes());
    args.push(
        process::context()
            .server_sock_path
            .load()
            .as_os_str()
            .as_encoded_bytes()
            .to_vec(),
    );

    args.push(String::from("--init-vfd-table").into_bytes());
    args.push(crate::vfd::export_table()?.into_bytes());

    for env in envp {
        args.push(String::from("--env").into_bytes());
        args.push(env.to_vec());
    }

    if let Some(arg0) = argv.next() {
        args.push(String::from("--arg0").into_bytes());
        args.push(arg0.to_vec());
    }
    args.push(path.to_vec());
    args.push(String::from("--").into_bytes());
    for arg in argv {
        args.push(arg.to_vec());
    }

    Err(std::process::Command::new(mactux_exec)
        .args(
            args.into_iter()
                .map(|x| unsafe { OsString::from_encoded_bytes_unchecked(x) }),
        )
        .exec()
        .into())
}

pub fn fork() -> Result<i32, LxError> {
    let new_client = crate::ipc_client::make_client();

    let status = may_fork(
        || unsafe {
            match libc::fork() {
                ..0 => Err(LxError::last_apple_error()),
                0 => Ok(0),
                n => Ok(n),
            }
        },
        |x| matches!(x, Ok(0)),
    )?;

    if status == 0 {
        prepare_new_process(new_client);
    }

    Ok(status)
}

pub fn clone(args: CloneArgs) -> Result<i32, LxError> {
    let result = match args.flags().child_type() {
        ChildType::Process => fork(),
        ChildType::Thread => crate::thread::clone(args.clone()),
        ChildType::Unsupported => Err(LxError::EINVAL),
    };
    match result {
        Ok(0) => {
            if args.flags().contains(CloneFlags::CLONE_SETTLS) {
                crate::emuctx::x86_64_set_emulated_gsbase(args.tls());
            }
        }
        Ok(child_tid) => unsafe {
            if args.flags().contains(CloneFlags::CLONE_PARENT_SETTID) {
                args.parent_tid().write(child_tid);
            }
        },
        Err(_) => {}
    };
    result
}

pub fn kill(pid: i32, signum: SigNum) -> Result<(), LxError> {
    let pid = match pid {
        0 => 0,
        -1 => -1,
        1.. => with_pid_mapper(|x| x.linux_to_apple(pid))?,
        ..0 => -with_pid_mapper(|x| x.linux_to_apple(-pid))?,
    };
    if is_tid(pid.abs()) {
        // TODO
        return Err(LxError::EPERM);
    }
    unsafe { posix_result(libc::kill(pid, signum.to_apple()?)) }
}

/// Does preparation work for the newly-created process.
fn prepare_new_process(client: Client) {
    if client.invoke(Request::AfterFork(pid())).is_err() {
        crate::error_report::fast_fail();
    }
    crate::ipc_client::update_client(client);
}
