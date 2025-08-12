use loader::Program;
use mimalloc::MiMalloc;
use std::{
    ffi::OsString,
    os::fd::{FromRawFd, OwnedFd},
    path::PathBuf,
};
use structures::fs::OpenFlags;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[derive(Debug, clap::Parser)]
struct Mactux {
    #[arg(short = 'd', long)]
    data_dir: Option<PathBuf>,

    #[arg(long)]
    init_sock_fd: Option<std::ffi::c_int>,

    #[arg(long)]
    init_vfd_table: Option<String>,

    #[arg(long)]
    cwd: Option<OsString>,

    exec: OsString,

    #[arg(long)]
    arg0: Option<OsString>,

    #[arg(last = true)]
    args: Vec<OsString>,

    #[arg(short, long)]
    env: Vec<OsString>,
}

fn main() {
    let cmdline: Mactux = clap::Parser::parse();

    setup_environment();
    if let Some(path) = &cmdline.data_dir {
        rtenv::ipc_client::set_server_sock_path(path.join("mactux.sock"));
    }
    if let Some(fd) = cmdline.init_sock_fd {
        unsafe {
            rtenv::ipc_client::set_client_fd(fd);
        }
    }
    if let Some(cwd) = &cmdline.cwd {
        if let Err(err) = rtenv::fs::chdir(cwd.clone().into_encoded_bytes()) {
            eprintln!("mactux: failed to initalize working directory: {err:?}");
            std::process::exit(1);
        }
    }
    if let Some(table) = &cmdline.init_vfd_table {
        if let Err(err) = rtenv::vfd::fill_table(table) {
            eprintln!("mactux: failed to initalize vfd table: {err:?}");
            std::process::exit(1);
        }
    }

    let args = collect_args(&cmdline);
    let envp = collect_envp(&cmdline);
    let prog = load_program(cmdline.exec.as_encoded_bytes());
    unsafe {
        prog.run(args.iter().copied(), envp.iter().copied());
    }
}

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

fn load_program(exec: &[u8]) -> Program {
    let fd = rtenv::fs::open(exec.to_vec(), OpenFlags::O_CLOEXEC | OpenFlags::O_RDONLY, 0)
        .unwrap_or_else(|err| {
            eprintln!(
                "mactux: failed to open executable file \"{}\": {:?}",
                String::from_utf8_lossy(exec),
                err
            );
            std::process::exit(101);
        });
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

fn collect_envp(cmdline: &Mactux) -> Vec<&[u8]> {
    let mut envp = Vec::with_capacity(cmdline.env.len());
    cmdline
        .env
        .iter()
        .for_each(|x| envp.push(x.as_encoded_bytes()));
    envp
}
