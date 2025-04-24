// SPDX-FileCopyrightText: © 2021 The gfx-rs developers
// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
//
// See lib.rs for more information about the initial source.

extern "C" {
    fn webgpu_compute_pipeline_get_bind_group_layout(instance_id: u64, index: u32) -> u64;
}

#[derive(Debug)]
pub struct EdgeGpuComputePipeline {
    pub(crate) ident: u64,
}

impl Drop for EdgeGpuComputePipeline {
    fn drop(&mut self) {
        unsafe {
            crate::webgpu_drop(self.ident);
        }
    }
}

impl wgpu::custom::ComputePipelineInterface for EdgeGpuComputePipeline {
    fn get_bind_group_layout(&self, index: u32) -> wgpu::custom::DispatchBindGroupLayout {
        let bind_group_ident = unsafe { webgpu_compute_pipeline_get_bind_group_layout(self.ident, index) };

        wgpu::custom::DispatchBindGroupLayout::custom(crate::EdgeGpuBindGroupLayout { ident: bind_group_ident })
    }
}
