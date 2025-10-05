//! Access to the working directory of the server.

use anyhow::anyhow;
use std::path::PathBuf;

/// The working directory of the server.
#[derive(Debug)]
pub struct WorkDir(pub PathBuf);
impl WorkDir {
    /// Initializes this working directory.
    pub fn init(&self) -> anyhow::Result<()> {
        if self.init_flag().exists() {
            return Ok(());
        }
        self.force_init()
    }

    /// Returns path of the init flag file.
    pub fn init_flag(&self) -> PathBuf {
        self.0.join("init_flag")
    }

    /// Returns path of the emulated root filesystem path.
    pub fn rootfs(&self) -> PathBuf {
        self.0.join("rootfs")
    }

    /// Returns path of the IPC socket file.
    pub fn ipc_socket(&self) -> PathBuf {
        self.0.join("mactux.sock")
    }

    /// Forces initialization of this working directory.
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

    /// Initializes the default root filesystem.
    fn init_rootfs(&self) -> anyhow::Result<()> {
        tracing::info!("Initializing default root filesystem...");
        std::fs::create_dir(self.rootfs())?;
        tracing::info!("Initialized default root filesystem...");
        Ok(())
    }
}

/// Returns the default working directory path, or exits on error.
pub fn force_default_path() -> PathBuf {
    match default_path() {
        Ok(x) => x,
        Err(err) => {
            tracing::error!("{err}");
            std::process::exit(1);
        }
    }
}

/// Returns the default working directory path.
pub fn default_path() -> anyhow::Result<PathBuf> {
    Ok(std::env::home_dir()
        .ok_or_else(|| anyhow!("failed to query home directory"))?
        .join(".mactux"))
}
