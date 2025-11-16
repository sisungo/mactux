use loader::Program;
use mimalloc::MiMalloc;
use std::{
    ffi::OsString,
    os::fd::{FromRawFd, OwnedFd},
    path::PathBuf,
};
use structures::fs::{AT_FDCWD, AtFlags, FileMode, OpenFlags};

/// Specifies [`MiMalloc`] as memory allocator.
///
/// We have to do some dynamic memory allocations in our signal handler. The allocator is lock-free, and works well in
/// signal handlers.
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[derive(Debug, clap::Parser)]
struct Mactux {
    /// Specify path of the server socket
    #[arg(long)]
    server_sock_path: Option<PathBuf>,

    /// Specify fd number of server socket of the initial thread
    #[arg(long)]
    init_sock_fd: Option<std::ffi::c_int>,

    /// Initial VFD table mapping
    #[arg(long)]
    init_vfd_table: Option<String>,

    /// Initial current working directory
    #[arg(long)]
    cwd: Option<OsString>,

    /// Path of the binary to execute
    exec: OsString,

    /// The `0`-th argument
    #[arg(long)]
    arg0: Option<OsString>,

    /// Arguments passed to the program
    #[arg(last = true)]
    args: Vec<OsString>,

    /// Environment variables passed to the program
    #[arg(short, long)]
    env: Vec<OsString>,
}

fn main() {
    let cmdline: Mactux = clap::Parser::parse();

    setup_environment();
    if let Some(path) = &cmdline.server_sock_path {
        rtenv::ipc_client::set_server_sock_path(path.clone());
    }
    if let Some(fd) = cmdline.init_sock_fd {
        unsafe {
            rtenv::ipc_client::set_client_fd(fd);
        }
    }
    if let Some(cwd) = &cmdline.cwd
        && let Err(err) = rtenv::fs::init_cwd(cwd.clone().into_encoded_bytes())
    {
        eprintln!("mactux: failed to initialize cwd: {err:?}",);
        std::process::exit(1);
    }
    if let Some(table) = &cmdline.init_vfd_table
        && let Err(err) = rtenv::vfd::fill_table(table)
    {
        eprintln!("mactux: failed to initalize vfd table: {err:?}");
        std::process::exit(1);
    }

    let args = collect_args(&cmdline);
    let envp = collect_envp(&cmdline);
    let prog = load_program(cmdline.exec.as_encoded_bytes());
    unsafe {
        prog.run(&args, &envp);
    }
}

/// Initializes the environmental libraries.
fn setup_environment() {
    if let Err(err) = std::env::set_current_dir("/") {
        eprintln!("mactux: failed to switch to secure path \"/\": {err}");
        std::process::exit(101);
    }

    unsafe {
        rtenv::install().unwrap();
        syscall::install().unwrap();
    }
}

/// Loads a program at the given path.
fn load_program(exec: &[u8]) -> Program {
    let fd = rtenv::fs::openat(
        AT_FDCWD,
        exec.to_vec(),
        OpenFlags::O_CLOEXEC | OpenFlags::O_RDONLY,
        AtFlags::empty(),
        FileMode(0),
    );
    let fd = match fd {
        Ok(x) => x,
        Err(e) => {
            eprintln!(
                "mactux: failed to open executable file \"{}\": {:?}",
                String::from_utf8_lossy(exec),
                e
            );
            std::process::exit(101);
        }
    };
    if rtenv::vfd::get(fd).is_some() {
        _ = rtenv::io::close(fd);
        eprintln!("mactux: virtual file descriptors are not yet supported to be executed");
        std::process::exit(101);
    }
    loader::Program::load(unsafe { OwnedFd::from_raw_fd(fd) }).unwrap_or_else(|err| {
        eprintln!(
            "mactux: failed to load executable file \"{}\": {}",
            String::from_utf8_lossy(exec),
            err
        );
        std::process::exit(101);
    })
}

/// Collects arguments from `cmdline`.
fn collect_args(cmdline: &Mactux) -> Vec<&[u8]> {
    let mut args = Vec::with_capacity(cmdline.args.len() + 1);
    let arg0 = cmdline.arg0.as_ref().unwrap_or(&cmdline.exec);
    args.push(arg0.as_encoded_bytes());
    cmdline
        .args
        .iter()
        .for_each(|x| args.push(x.as_encoded_bytes()));
    args
}

/// Collects environmental variables from `cmdline`.
fn collect_envp(cmdline: &Mactux) -> Vec<&[u8]> {
    let mut envp = Vec::with_capacity(cmdline.env.len());
    cmdline
        .env
        .iter()
        .for_each(|x| envp.push(x.as_encoded_bytes()));
    envp
}
