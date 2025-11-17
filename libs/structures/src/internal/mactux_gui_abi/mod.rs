pub mod gpu;
pub mod gui;

use crate::error::LxError;
use bincode::{Decode, Encode};
use std::io::Write;

pub type MethodName = [u8; 3];

#[derive(Debug, Clone, Encode, Decode)]
pub struct RequestHeader {
    pub id: u64,
    pub method: MethodName,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct ResponseHeader {
    pub id: u64,
    pub status: i32,
}

pub fn respond<P: Encode>(wr: &mut impl Write, hdr: ResponseHeader, pld: P) -> Result<(), LxError> {
    bincode::encode_into_std_write(hdr, wr, bincode::config::standard())
        .map_err(|_| LxError::EIO)?;
    bincode::encode_into_std_write(pld, wr, bincode::config::standard())
        .map_err(|_| LxError::EIO)?;
    Ok(())
}
