use anyhow::anyhow;
use std::path::PathBuf;

#[derive(Debug)]
pub struct WorkDir(pub PathBuf);
impl WorkDir {
    pub fn init(&self) -> anyhow::Result<()> {
        if self.init_flag().exists() {
            return Ok(());
        }
        self.force_init()
    }

    pub fn init_flag(&self) -> PathBuf {
        self.0.join("init_flag")
    }

    pub fn rootfs(&self) -> PathBuf {
        self.0.join("rootfs")
    }

    pub fn ipc_socket(&self) -> PathBuf {
        self.0.join("mactux.sock")
    }

    fn force_init(&self) -> anyhow::Result<()> {
        tracing::info!("Initializing work directory \"{}\"...", self.0.display());
        if !self.0.exists() {
            std::fs::create_dir(&self.0)?;
        }
        self.init_rootfs()?;
        std::fs::File::create_new(self.init_flag())?;
        tracing::info!("Work directory initialized.");
        Ok(())
    }

    fn init_rootfs(&self) -> anyhow::Result<()> {
        tracing::info!("Initializing default root filesystem...");
        std::fs::create_dir(self.rootfs())?;
        tracing::info!("Initialized default root filesystem...");
        Ok(())
    }
}

pub fn force_default_path() -> PathBuf {
    match default_path() {
        Ok(x) => x,
        Err(err) => {
            tracing::error!("{err}");
            std::process::exit(1);
        }
    }
}

pub fn default_path() -> anyhow::Result<PathBuf> {
    Ok(std::env::home_dir()
        .ok_or_else(|| anyhow!("failed to query home directory"))?
        .join(".mactux"))
}
