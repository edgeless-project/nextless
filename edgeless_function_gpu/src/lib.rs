// SPDX-FileCopyrightText: © 2021 The gfx-rs developers
// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
//
//
// Based on
// https://github.com/gfx-rs/wgpu/blob/trunk/examples/standalone/custom_backend/src/custom.rs &
// https://github.com/gfx-rs/wgpu/blob/trunk/wgpu/src/backend/webgpu.rs (MIT/APACHE Licensed).

pub use wgpu;

extern "C" {
    fn webgpu_cp_drop(compute_pass_id: u64);
    fn webgpu_drop(instance_id: u64);
}

mod instance;
pub use instance::EdgeGpuInstance;

mod adapter;
pub use adapter::EdgeGpuAdapter;

mod device;
pub use device::EdgeGpuDevice;

mod queue;
pub use queue::EdgeGpuQueue;

mod shader_module;
pub use shader_module::EdgeGpuShaderModule;

mod bind_group_layout;
pub use bind_group_layout::EdgeGpuBindGroupLayout;

mod pipeline_layouts;
pub use pipeline_layouts::EdgeGpuPipelineLayout;

mod bind_group;
pub use bind_group::EdgeGpuBindGroup;

mod compute_pipeline;
pub use compute_pipeline::EdgeGpuComputePipeline;

mod buffer;
pub use buffer::EdgeGpuBuffer;

mod command_encoder;
pub use command_encoder::EdgeGpuCommandEncoder;

mod command_buffer;
pub use command_buffer::EdgeGpuCommandBuffer;

mod compute_pass;
pub use compute_pass::EdgeGpuComputePass;

mod mapped_range;
pub use mapped_range::EdgeGpuMappedRange;
