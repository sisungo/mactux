pub mod gpu;
pub mod gui;

use serde::{Deserialize, Serialize};

pub type MethodName = [u8; 3];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestHeader {
    pub id: u64,
    pub method: MethodName,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseHeader {
    pub id: u64,
    pub status: i32,
}
