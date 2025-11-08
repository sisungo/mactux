//! Configuration of the application.

use anyhow::anyhow;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct WorkDir(PathBuf);
impl WorkDir {
    pub fn new(path: PathBuf) -> anyhow::Result<Self> {
        let this = Self(path);
        if !this.init_flag().exists() {
            init_work_dir(&this)?;
        }
        _ = std::fs::remove_dir_all(this.net());
        std::fs::create_dir(this.net())?;
        Ok(this)
    }

    pub fn try_default() -> anyhow::Result<Self> {
        Self::new(
            std::env::home_dir()
                .ok_or_else(|| anyhow!("unknown home directory"))?
                .join(".mactux"),
        )
    }

    pub fn sock(&self) -> PathBuf {
        self.0.join("mactux.sock")
    }

    pub fn rootfs(&self) -> PathBuf {
        self.0.join("rootfs")
    }

    pub fn init_flag(&self) -> PathBuf {
        self.0.join("init_flag")
    }

    pub fn net(&self) -> PathBuf {
        self.0.join("net")
    }
}

fn init_work_dir(dir: &WorkDir) -> anyhow::Result<()> {
    eprintln!(
        "mactux_server: initializing working directory at {}",
        dir.0.display()
    );

    std::fs::create_dir_all(dir.rootfs())?;
    std::fs::File::create_new(dir.init_flag())?;

    Ok(())
}
