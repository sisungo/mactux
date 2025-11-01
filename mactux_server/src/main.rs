#![feature(peer_credentials_unix_socket)]

mod config;
mod device;
mod filesystem;
mod ipc;
mod multimedia;
mod network;
mod poll;
mod sysinfo;
mod syslog;
mod task;
mod util;
mod vfd;

use crate::{
    config::WorkDir,
    device::DeviceTable,
    filesystem::{VPath, vfs::MountNamespace},
    sysinfo::{InitUts, UtsNamespace},
    syslog::Syslog,
    task::{InitPid, PidNamespace, process::Process, thread::Thread},
    util::{ReclaimRegistry, Shared},
    vfd::VfdTable,
};
use anyhow::{Context, anyhow};
use std::{path::PathBuf, sync::OnceLock};

static APP: OnceLock<App> = OnceLock::new();

/// Global application state.
struct App {
    /// The working directory.
    work_dir: WorkDir,

    /// Registry of all Linux processes, indexed by native PID.
    processes: ReclaimRegistry<Process>,

    /// Registry of all Linux threads, indexed by thread ID.
    threads: ReclaimRegistry<Thread>,

    /// Registry of all devices.
    devices: DeviceTable,

    /// Namespaces.
    namespaces: Namespaces,

    /// The system logger.
    syslog: Syslog,

    /// The server thread.
    server_thread: OnceLock<Shared<Thread>>,
}
impl App {
    fn new(cli: Cli) -> anyhow::Result<Self> {
        let processes = ReclaimRegistry::new();
        let threads = ReclaimRegistry::new();
        let work_dir = match cli.work_dir {
            Some(dir) => WorkDir::new(dir)?,
            None => WorkDir::try_default()?,
        };
        Ok(Self {
            work_dir,
            processes,
            threads,
            devices: DeviceTable::new(),
            namespaces: Namespaces::new(),
            syslog: Syslog::new(),
            server_thread: OnceLock::new(),
        })
    }

    fn run(&'static self) -> anyhow::Result<()> {
        ipc::Listener::new(self.work_dir.sock())
            .context("failed to create ipc socket")?
            .start();

        loop {
            std::thread::park();
        }
    }
}

/// Namespace registries.
struct Namespaces {
    /// Registry of all mount namespaces.
    mount: ReclaimRegistry<MountNamespace>,

    /// Registry of all PID namespaces.
    pid: ReclaimRegistry<Box<dyn PidNamespace>>,

    /// Registry of all UTS namespaces.
    uts: ReclaimRegistry<Box<dyn UtsNamespace>>,

    /// The initial mount namespace.
    init_mnt: OnceLock<Shared<MountNamespace>>,

    /// The initial PID namespace.
    init_pid: OnceLock<Shared<Box<dyn PidNamespace>>>,

    /// The initial UTS namespace.
    init_uts: OnceLock<Shared<Box<dyn UtsNamespace>>>,
}
impl Namespaces {
    fn new() -> Self {
        Self {
            mount: ReclaimRegistry::new(),
            pid: ReclaimRegistry::new(),
            uts: ReclaimRegistry::new(),
            init_mnt: OnceLock::new(),
            init_pid: OnceLock::new(),
            init_uts: OnceLock::new(),
        }
    }

    fn init(&'static self) {
        let init_mnt = self.mount.register(MountNamespace::new());
        assert_eq!(Shared::id(&init_mnt), 1);
        _ = self.init_mnt.set(init_mnt);

        let init_pid = self.pid.register(Box::new(InitPid::new()));
        assert_eq!(Shared::id(&init_pid), 1);
        _ = self.init_pid.set(init_pid);

        let init_uts = self.uts.register(Box::new(InitUts));
        assert_eq!(Shared::id(&init_uts), 1);
        _ = self.init_uts.set(init_uts);
    }

    fn init_mnt(&self) -> Shared<MountNamespace> {
        self.init_mnt.get().unwrap().clone()
    }

    fn init_pid(&self) -> Shared<Box<dyn PidNamespace>> {
        self.init_pid.get().unwrap().clone()
    }

    fn init_uts(&self) -> Shared<Box<dyn UtsNamespace>> {
        self.init_uts.get().unwrap().clone()
    }
}

#[derive(clap::Parser)]
struct Cli {
    #[arg(short = 'd', long)]
    work_dir: Option<PathBuf>,
}

fn main() {
    let cli: Cli = clap::Parser::parse();

    if let Err(err) = init_app(cli) {
        eprintln!("mactux_server: cannot initialize application: {err}");
        std::process::exit(1);
    }

    if let Err(err) = init_env() {
        eprintln!("mactux_server: cannot initialize Linux environment: {err}");
        std::process::exit(1);
    }

    syslog::install_rust().expect("syslog::install_rust called twice");

    if let Err(err) = app().run() {
        log::error!("cannot run application: {err}");
        std::process::exit(1);
    }
}

/// Initializes the global application state.
///
/// Some initializations requires to be set here, instead of [`App::new`] and simply [`OnceLock::set`], because they
/// use [`ReclaimRegistry`]s, which requires a static lifetime.
fn init_app(cli: Cli) -> anyhow::Result<()> {
    if APP.set(App::new(cli)?).is_err() {
        return Err(anyhow!("init_app is called twice"));
    }

    app().namespaces.init();

    let server_proc: Shared<Process> = app().processes.intervene(
        std::process::id() as _,
        Process {
            mnt: app().namespaces.init_mnt(),
            uts: app().namespaces.init_uts(),
            vfd: VfdTable::new(),
            pid: app().namespaces.init_pid(),
        },
    );
    let server_thrd = Thread::builder().process(server_proc).is_main().build()?;
    _ = app().server_thread.set(server_thrd);

    Ok(())
}

/// Initializes the Linux environment, like mounts listed in `/etc/fstab`.
///
/// We tend to initialize the most thing inside the Linux environment, however, we need some initializations
/// to ensure a simple Linux program could be executed. Thus, we put them here.
fn init_env() -> anyhow::Result<()> {
    app().devices.discover();
    init_mounts()?;
    Ok(())
}

/// Initializes mounts listed in `/etc/fstab`.
fn init_mounts() -> anyhow::Result<()> {
    let fstab = app().work_dir.rootfs().join("etc/fstab");
    let fstab =
        std::fs::read_to_string(&fstab).context(format!("failed to read {}", fstab.display()))?;
    let fstab = fstab.parse::<structures::convention::Fstab>()?;
    let init_mnt = app().namespaces.init_mnt();
    let rootfs_source = format!("native={}", app().work_dir.rootfs().display());
    init_mnt.mount(
        rootfs_source.as_bytes(),
        &VPath::parse(b"/"),
        "nativefs",
        0,
        0,
    )?;
    for entry in fstab.0 {
        init_mnt.mount(
            entry.device.as_bytes(),
            &VPath::parse(entry.mount_point.as_bytes()),
            &entry.fs_type,
            0,
            0,
        )?;
        // TODO Support mount flags
    }
    Ok(())
}

/// Returns a reference to the global application state.
fn app() -> &'static App {
    APP.get().unwrap()
}
