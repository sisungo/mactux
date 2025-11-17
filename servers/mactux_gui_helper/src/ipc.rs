use std::fmt::Debug;
use structures::internal::mactux_gui_abi::MethodName;

pub type MethodIndexBox<T> = [Option<Box<T>>; 256];
pub type FnIpc = dyn Fn(u64);

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
