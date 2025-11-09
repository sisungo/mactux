use bincode::{Decode, Encode};

/// Network names of current UTS namespace.
#[derive(Debug, Clone, Encode, Decode)]
pub struct NetworkNames {
    pub nodename: Vec<u8>,
    pub domainname: Vec<u8>,
}
