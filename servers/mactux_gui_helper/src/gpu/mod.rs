pub mod gpu;

use crate::ipc::{FnIpc, MethodIndexBox};

pub fn ipc_classes() -> Box<MethodIndexBox<MethodIndexBox<FnIpc>>> {
    let mut this = Box::new([const { None }; _]);
    this[0] = Some(gpu::ipc_methods());
    this
}
