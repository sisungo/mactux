use std::{
    path::PathBuf,
    sync::atomic::{self, AtomicU64},
};
use structures::error::LxError;

#[derive(Debug)]
pub struct AbstractNamespace {
    path: PathBuf,
    next_id: AtomicU64,
}
impl AbstractNamespace {
    pub fn new(path: PathBuf) -> std::io::Result<Self> {
        _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir(&path)?;

        Ok(Self {
            path,
            next_id: AtomicU64::new(1),
        })
    }

    pub fn create_named(&self, name: &[u8]) -> Result<u64, LxError> {
        let id = self.next_id.fetch_add(1, atomic::Ordering::Relaxed);
        let escaped = escape_abstract_name(name);
        let map_file = self.path.join(format!("{escaped}.map"));
        std::fs::write(map_file, id.to_string())?;
        Ok(id)
    }

    pub fn sock_by_id(&self, id: u64) -> PathBuf {
        self.path.join(format!("{id}.sock"))
    }

    pub fn id_by_name(&self, name: &[u8]) -> Result<u64, LxError> {
        let escaped = escape_abstract_name(name);
        let map_file = self.path.join(format!("{escaped}.map"));
        std::fs::read_to_string(&map_file)
            .map_err(|_| LxError::ENOENT)?
            .parse()
            .map_err(|_| LxError::EIO)
    }

    pub fn sock_by_name(&self, name: &[u8]) -> Result<PathBuf, LxError> {
        Ok(self.sock_by_id(self.id_by_name(name)?))
    }
}

fn escape_abstract_name(before: &[u8]) -> String {
    let mut after = String::with_capacity(before.len());
    for &ch in before {
        match ch {
            b'+' => after.push_str("++"),
            b'/' => after.push_str("+s"),
            b'@' => after.push_str("+@"),
            _ => (),
        }
        if ch.is_ascii() {
            after.push(ch as char);
        } else {
            after.push_str(&format!("@{ch}@"));
        }
    }
    after
}
