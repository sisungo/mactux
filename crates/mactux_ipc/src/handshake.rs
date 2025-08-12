//! Structures listed here would never change.

use bincode::{Decode, Encode};

#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HandshakeRequest {
    pub magic: [u8; 8],
}
impl HandshakeRequest {
    pub const MAGIC: [u8; 8] = *b"MACTUXHQ";

    pub fn new() -> Self {
        Self { magic: Self::MAGIC }
    }
}

#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HandshakeResponse {
    pub magic: [u8; 8],
    pub version: String,
}
impl HandshakeResponse {
    pub const MAGIC: [u8; 8] = *b"MACTUXHS";

    pub fn new() -> Self {
        Self {
            magic: Self::MAGIC,
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }
}
