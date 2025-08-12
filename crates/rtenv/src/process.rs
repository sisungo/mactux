use crate::{
    emuctx::may_fork,
    ipc_client::{Client, with_client},
    posix_bi,
};
use arc_swap::ArcSwap;
use mactux_ipc::request::Request;
use rustc_hash::FxBuildHasher;
use std::{
    convert::Infallible,
    ffi::{OsString, c_int},
    mem::MaybeUninit,
    os::{fd::AsRawFd, unix::process::CommandExt},
    sync::Arc,
};
use structures::{
    error::LxError,
    fs::AccessFlags,
    process::{ChildType, CloneArgs},
    signal::{SigAction, SigNum},
};

static mut PROCESS_CTX: MaybeUninit<ProcessCtx> = MaybeUninit::uninit();

#[derive(Debug)]
pub struct ProcessCtx {
    pub cwd: ArcSwap<Vec<u8>>,
    pub thread_info_map: crate::emuctx::ThreadInfoMap,
    pub sigactions: [ArcSwap<SigAction>; SigNum::_NSIG as usize],
    pub vfd_table: papaya::HashMap<c_int, u64, FxBuildHasher>,
}

pub unsafe fn install() -> std::io::Result<()> {
    let cwd = ArcSwap::from(Arc::new(vec![b'/']));
    let thread_info_map = crate::emuctx::ThreadInfoMap::new();
    let sigactions = std::array::from_fn(|_| ArcSwap::from(Arc::new(SigAction::new())));
    let vfd_table = papaya::HashMap::with_capacity_and_hasher(128, FxBuildHasher::default());
    unsafe {
        (*&raw mut PROCESS_CTX).as_mut_ptr().write(ProcessCtx {
            cwd,
            thread_info_map,
            sigactions,
            vfd_table,
        });
    }
    Ok(())
}

pub fn context() -> &'static ProcessCtx {
    unsafe { (*&raw const PROCESS_CTX).assume_init_ref() }
}

pub fn pid() -> i32 {
    // TODO: support namespaces
    unsafe { libc::getpid() }
}

pub fn ppid() -> i32 {
    // TODO: support namespaces
    unsafe { libc::getppid() }
}

pub fn pgid(pid: i32) -> i32 {
    // TODO: support namespaces
    unsafe { libc::getpgid(pid) }
}

pub fn setpgid(pid: i32, pgid: i32) -> Result<i32, LxError> {
    // TODO: support namespaces
    unsafe {
        match libc::setpgid(pid, pgid) {
            -1 => Err(LxError::last_apple_error()),
            n => Ok(n),
        }
    }
}

pub unsafe fn exec(
    path: &[u8],
    argv: &[*const u8],
    envp: &[*const u8],
) -> Result<Infallible, LxError> {
    if crate::fs::access(path.to_vec(), AccessFlags::F_OK).is_err() {
        return Err(LxError::ENOENT);
    }
    if crate::fs::access(path.to_vec(), AccessFlags::X_OK).is_err() {
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

    args.push(String::from("--cwd").into_bytes());
    args.push(crate::fs::getcwd());

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
    new_client.invoke(Request::BeforeFork).unwrap();

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
    match args.flags().child_type() {
        ChildType::Process => todo!(),
        ChildType::Thread => todo!(),
        ChildType::Unsupported => Err(LxError::EINVAL),
    }
}

pub fn kill(pid: i32, signum: SigNum) -> Result<(), LxError> {
    // TODO
    unsafe { posix_bi!(libc::kill(pid, signum.to_apple()?)) }
}

fn prepare_new_process(client: Client) {
    if client.invoke(Request::AfterFork(pid())).is_err() {
        crate::error_report::fast_fail();
    }
    crate::ipc_client::update_client(client);
}
