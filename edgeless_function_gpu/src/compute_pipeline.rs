// SPDX-FileCopyrightText: © 2021 The gfx-rs developers
// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
//
// See lib.rs for more information about the initial source.

#[derive(Debug)]
pub struct EdgeGpuComputePipeline {
    pub(crate) ident: u64,
}

impl wgpu::custom::ComputePipelineInterface for EdgeGpuComputePipeline {
    fn get_bind_group_layout(&self, _index: u32) -> wgpu::custom::DispatchBindGroupLayout {
        log::error!("Tried to get Layout");
        panic!()
    }
}
