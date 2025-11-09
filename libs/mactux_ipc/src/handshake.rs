//! Structures listed here would never change.

use bincode::{Decode, Encode};

/// A handshake request.
#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HandshakeRequest {
    pub magic: [u8; 8],
}
impl HandshakeRequest {
    /// The magic number.
    pub const MAGIC: [u8; 8] = *b"MACTUXHQ";

    /// Creates a new [`HandshakeRequest`] instance, in its only valid form.
    pub fn new() -> Self {
        Self { magic: Self::MAGIC }
    }
}

/// A handshake response.
#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HandshakeResponse {
    pub magic: [u8; 8],
    pub version: String,
}
impl HandshakeResponse {
    /// The magic number.
    pub const MAGIC: [u8; 8] = *b"MACTUXHS";

    /// Creates a new [`HandshakeResponse`] instance that fits current library version.
    pub fn new() -> Self {
        Self {
            magic: Self::MAGIC,
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }
}
