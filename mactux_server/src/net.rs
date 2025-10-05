//! Server-assisted complex networking features support.

use std::{
    path::PathBuf,
    sync::atomic::{self, AtomicU64},
};
use structures::error::LxError;

/// An abstract namespace.
///
/// Note that this is the manager of all Unix sockets, not only the ones in the abstract namespace. This
/// design avoids problems caused by name length limits, which diverse between macOS and Linux, and may
/// cause confusion with our own VFS.
#[derive(Debug)]
pub struct AbstractNamespace {
    path: PathBuf,
    next_id: AtomicU64,
}
impl AbstractNamespace {
    /// Creates a new abstract namespace in the given directory.
    ///
    /// This will delete any existing contents in the directory if it exists, or the directory is created
    /// if it does not exist.
    pub fn new(path: PathBuf) -> std::io::Result<Self> {
        _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir(&path)?;

        Ok(Self {
            path,
            next_id: AtomicU64::new(1),
        })
    }

    /// Creates a new abstract socket with the given name, returning its ID.
    pub fn create_named(&self, name: &str) -> Result<u64, LxError> {
        let id = self.next_id.fetch_add(1, atomic::Ordering::Relaxed);
        let escaped = escape_abstract_name(name);
        let map_file = self.path.join(format!("{escaped}.map"));
        std::fs::write(map_file, id.to_string())?;
        Ok(id)
    }

    /// Returns the socket path for the given ID.
    pub fn sock_by_id(&self, id: u64) -> PathBuf {
        self.path.join(format!("{id}.sock"))
    }

    /// Returns the ID for the given name.
    pub fn id_by_name(&self, name: &str) -> Result<u64, LxError> {
        let escaped = escape_abstract_name(name);
        let map_file = self.path.join(format!("{escaped}.map"));
        std::fs::read_to_string(&map_file)
            .map_err(|_| LxError::ENOENT)?
            .parse()
            .map_err(|_| LxError::EIO)
    }

    /// Returns the socket path for the given name.
    pub fn sock_by_name(&self, name: &str) -> Result<PathBuf, LxError> {
        Ok(self.sock_by_id(self.id_by_name(name)?))
    }
}

/// Escapes a name to be used as a filename in the abstract namespace.
fn escape_abstract_name(before: &str) -> String {
    before.to_string()
}
