//! Filesystem implementations and abstractions.

pub mod eventfd;
pub mod invalidfd;
pub mod nativefs;
pub mod procfs;
pub mod tmpfs;
pub mod vfs;

use std::fmt::Debug;
use structures::error::LxError;

#[derive(Clone, PartialEq, Eq)]
pub struct VPath {
    pub slash_prefix: bool,
    pub parts: Vec<Vec<u8>>,
    pub slash_suffix: bool,
}
impl VPath {
    /// Parses a path in bytes into a [`VPath`].
    pub fn parse(bytes: &[u8]) -> Self {
        let slash_prefix = bytes.first() == Some(&b'/');
        let slash_suffix = bytes.last() == Some(&b'/');
        let parts = bytes
            .split(|b| *b == b'/')
            .filter(|p| !p.is_empty())
            .map(|p| p.to_vec())
            .collect();
        Self {
            slash_prefix,
            parts,
            slash_suffix,
        }
    }

    pub fn clearize(&self) -> Result<Self, LxError> {
        if !self.slash_prefix {
            return Err(LxError::EINVAL);
        }

        let mut dst_parts = Vec::with_capacity(self.parts.len());

        for i in &self.parts {
            if &i[..] == &b".."[..] {
                dst_parts.pop();
            } else if &i[..] == &b"."[..] {
                continue;
            } else if !i.is_empty() {
                dst_parts.push(i.clone());
            }
        }

        Ok(Self {
            slash_prefix: true,
            slash_suffix: self.slash_suffix && !dst_parts.is_empty(),
            parts: dst_parts,
        })
    }

    /// Converts the path to a human-readable format.
    pub fn express(&self) -> Vec<u8> {
        let mut result = Vec::new();
        if self.slash_prefix {
            result.push(b'/');
        }
        for (n, i) in self.parts.iter().enumerate() {
            i.iter().for_each(|ch| result.push(*ch));
            if n != self.parts.len() - 1 {
                result.push(b'/');
            }
        }
        if self.slash_suffix {
            result.push(b'/');
        }
        result
    }
}
impl Debug for VPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VPath")
            .field("slash_prefix", &self.slash_prefix)
            .field(
                "parts",
                &self
                    .parts
                    .iter()
                    .map(|x| String::from_utf8_lossy(x))
                    .collect::<Vec<_>>(),
            )
            .field("slash_suffix", &self.slash_suffix)
            .finish()
    }
}
