use std::{
    fmt::Debug,
    io::{Read, Write},
};
use structures::{
    error::LxError,
    internal::mactux_gui_abi::{MethodName, ResponseHeader},
};

pub type MethodIndexBox<T> = [Option<Box<T>>; 256];
pub type FnIpc = dyn Fn(u64, &mut dyn Stream);

/// Index of registered methods.
pub struct MethodIndex(Box<MethodIndexBox<MethodIndexBox<MethodIndexBox<FnIpc>>>>);
impl MethodIndex {
    pub fn new() -> Self {
        let mut this = Self(Box::new([const { None }; _]));
        this.0[2] = Some(crate::gpu::ipc_classes());
        this
    }

    pub fn get(&self, name: MethodName) -> Option<&FnIpc> {
        self.0[name[0] as usize].as_ref()?[name[1] as usize].as_ref()?[name[2] as usize].as_deref()
    }
}
impl Debug for MethodIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map().finish()
    }
}

pub trait Stream: Read + Write {}
impl<T: Read + Write> Stream for T {}

pub trait Respond {
    fn status(&self) -> i32;
    fn write_body(self, stream: &mut dyn Stream) -> std::io::Result<()>;
}
impl Respond for () {
    fn status(&self) -> i32 {
        0
    }

    fn write_body(self, _stream: &mut dyn Stream) -> std::io::Result<()> {
        Ok(())
    }
}
impl<T> Respond for Result<T, LxError>
where
    T: Respond,
{
    fn status(&self) -> i32 {
        match self {
            Ok(_) => 0,
            Err(e) => -(e.0 as i32),
        }
    }

    fn write_body(self, stream: &mut dyn Stream) -> std::io::Result<()> {
        match self {
            Ok(body) => body.write_body(stream),
            Err(_) => Ok(()),
        }
    }
}

pub fn _handle_proto_error(mut stream: &mut dyn Write, id: u64) {
    let resp = ResponseHeader {
        id,
        status: -(LxError::EINVAL.0 as i32),
    };
    _ = bincode::encode_into_std_write(&resp, &mut stream, bincode::config::standard());
}
