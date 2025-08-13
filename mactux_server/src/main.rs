mod device;
mod filesystem;
mod net;
mod process;
mod server;
mod sysinfo;
mod syslog;
mod util;
mod uts;
mod vfd;
mod work_dir;

#[cfg(feature = "audio")]
mod audio;

use crate::{
    filesystem::vfs::{MountNamespace, VfsPath},
    process::ProcessCtx,
    util::Registry,
    work_dir::WorkDir,
};
use rustc_hash::FxBuildHasher;
use std::{
    fmt::Debug,
    path::PathBuf,
    sync::{Arc, OnceLock},
};
use structures::convention::Fstab;

static APP: OnceLock<Arc<App>> = OnceLock::new();

struct App {
    work_dir: WorkDir,
    mnt_ns_registry: Registry<Arc<MountNamespace>>,
    native_procs: papaya::HashMap<libc::pid_t, Arc<ProcessCtx>, FxBuildHasher>,
}
impl App {
    async fn new(cmdline: Cmdline) -> anyhow::Result<Arc<Self>> {
        let work_dir = WorkDir(
            cmdline
                .work_dir
                .unwrap_or_else(work_dir::force_default_path),
        );
        work_dir.init()?;

        let mnt_ns_registry = Registry::new();
        let init_mnt = MountNamespace::initial();
        init_mounts(&work_dir, &init_mnt).await?;
        assert_eq!(mnt_ns_registry.register(init_mnt), 1);

        let native_procs = papaya::HashMap::with_capacity_and_hasher(128, FxBuildHasher::default());
        Ok(Arc::new(Self {
            work_dir,
            mnt_ns_registry,
            native_procs,
        }))
    }

    async fn wait_for_exit(&self) {
        loop {
            if tokio::signal::ctrl_c().await.is_ok() {
                _ = std::fs::remove_file(self.work_dir.ipc_socket());
                break;
            }
        }
    }

    fn start(&self) -> anyhow::Result<()> {
        self.start_server()?;
        Ok(())
    }

    fn start_server(&self) -> anyhow::Result<()> {
        let sock_path = self.work_dir.ipc_socket();
        let server = server::Server::bind(&sock_path)?;
        tokio::spawn(server.run());
        Ok(())
    }
}
impl Debug for App {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("App").finish()
    }
}

#[derive(Debug, clap::Parser)]
struct Cmdline {
    #[arg(short = 'd', long)]
    work_dir: Option<PathBuf>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let cmdline: Cmdline = clap::Parser::parse();
    let app = match App::new(cmdline).await {
        Ok(x) => x,
        Err(err) => {
            tracing::error!("{err}");
            std::process::exit(1);
        }
    };
    APP.set(app.clone()).unwrap();

    if let Err(err) = app.start() {
        tracing::error!("{err}");
        std::process::exit(1);
    }
    app.wait_for_exit().await;
}

async fn init_mounts(work_dir: &WorkDir, init_mnt: &MountNamespace) -> anyhow::Result<()> {
    init_mnt
        .mount(
            VfsPath::from_bytes(b"/"),
            Arc::new(filesystem::nativefs::NativeFs::new(work_dir.rootfs())?),
        )
        .await
        .map_err(|err| anyhow::anyhow!("Failed to mount root: {}", err.0))?;
    let fstab = work_dir.rootfs().join("etc/fstab");
    let fstab = std::fs::read_to_string(fstab)?;
    let fstab = fstab.parse::<Fstab>()?;
    for entry in fstab.0 {
        let mountable = filesystem::vfs::mountable(
            &entry.fs_type,
            filesystem::vfs::MountDev::Freeform(entry.device.into()),
            &entry.options,
        )
        .await
        .map_err(|err| anyhow::anyhow!("Failed to create mountable: {}", err.0))?;
        let mountpoint = VfsPath::from_bytes(entry.mount_point.as_bytes()).to_storable();
        init_mnt
            .mount(mountpoint, mountable)
            .await
            .map_err(|err| anyhow::anyhow!("Failed to mount: {}", err.0))?;
    }
    Ok(())
}

fn app() -> &'static App {
    APP.get().unwrap()
}
