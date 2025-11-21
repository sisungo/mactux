pub mod gpu;
pub mod gui;

use bincode::{Decode, Encode};

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
