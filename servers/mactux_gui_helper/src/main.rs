mod config;
mod gpu;
mod ipc;
mod local_window;
mod util;

use crate::{config::BootstrapConfig, gpu::gpu::GPU, ipc::MethodIndex, util::DescRegistry};

#[derive(Debug)]
struct App {
    desc: DescTable,
    methods: MethodIndex,
}
impl App {}

#[derive(Debug)]
struct DescTable {
    gpu: DescRegistry<GPU>,
}

fn main() -> anyhow::Result<()> {
    util::install_logger();
    let bootstrap_config = BootstrapConfig::acquire()?;
    Ok(())
}
