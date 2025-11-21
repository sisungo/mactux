use crate::ipc::{FnIpc, MethodIndexBox};
use wgpu::{BackendOptions, Backends, InstanceDescriptor, InstanceFlags, MemoryBudgetThresholds};

/// A `GPU` instance in the WebGPU standard.
#[derive(Debug)]
pub struct GPU(wgpu::Instance);
impl GPU {
    /// Creates a new [`GPU`] instance.
    pub fn new() -> Self {
        let instance_descriptor = InstanceDescriptor {
            backends: Backends::all(),
            flags: InstanceFlags::empty(),
            memory_budget_thresholds: MemoryBudgetThresholds::default(),
            backend_options: BackendOptions::from_env_or_default(),
        };
        Self(wgpu::Instance::new(&instance_descriptor))
    }
}

pub fn ipc_methods() -> Box<MethodIndexBox<FnIpc>> {
    let mut this = Box::new([const { None }; _]);
    this[0] = Some(Box::new(gpu_new) as _);
    this
}

#[macros::gui_helper_method]
fn gpu_new() {}
