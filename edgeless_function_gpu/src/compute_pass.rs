// SPDX-FileCopyrightText: © 2021 The gfx-rs developers
// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
//
// See lib.rs for more information about the initial source.

extern "C" {
    fn webgpu_cp_set_pipeline(compute_pass_id: u64, compute_pipeline_id: u64);
    fn webgpu_cp_set_bind_group(compute_pass_id: u64, index: u32, bind_group_id: u64, offsets_ptr: *const u8, offsets_len: u64);
    fn webgpu_cp_dispatch_workgroups(compute_pass_id: u64, x: u32, y: u32, z: u32);
    fn webgpu_cp_dispatch_workgroups_indirect(compute_pass_id: u64, indirect_buffer_id: u64, indirect_offset: u64);
    fn webgpu_cp_set_push_constants(compute_pass_id: u64, offset: u32, data_ptr: *const u8, data_len: u64);
}

#[derive(Debug)]
pub struct EdgeGpuComputePass {
    pub(crate) ident: u64,
}

impl Drop for EdgeGpuComputePass {
    fn drop(&mut self) {
        unsafe {
            crate::webgpu_cp_drop(self.ident);
        }
    }
}

impl wgpu::custom::ComputePassInterface for EdgeGpuComputePass {
    fn set_pipeline(&mut self, pipeline: &wgpu::custom::DispatchComputePipeline) {
        unsafe { webgpu_cp_set_pipeline(self.ident, pipeline.as_custom::<crate::EdgeGpuComputePipeline>().unwrap().ident) }
    }

    fn set_bind_group(&mut self, index: u32, bind_group: Option<&wgpu::custom::DispatchBindGroup>, offsets: &[wgpu::DynamicOffset]) {
        let bind_group = match bind_group {
            Some(g) => g.as_custom::<crate::EdgeGpuBindGroup>().unwrap().ident,
            None => 0,
        };

        let offsets = serde_json::to_vec(offsets).unwrap();

        unsafe { webgpu_cp_set_bind_group(self.ident, index, bind_group, offsets.as_ptr(), offsets.len() as u64) }
    }

    fn set_push_constants(&mut self, offset: u32, data: &[u8]) {
        unsafe { webgpu_cp_set_push_constants(self.ident, offset, data.as_ptr(), data.len() as u64) }
    }

    fn insert_debug_marker(&mut self, _label: &str) {
        log::info!("Ignored Debug Marker")
    }

    fn push_debug_group(&mut self, _group_label: &str) {
        log::info!("Ignored Push Debug Group")
    }

    fn pop_debug_group(&mut self) {
        log::info!("Ignored Pop Debug Group")
    }

    fn write_timestamp(&mut self, _query_set: &wgpu::custom::DispatchQuerySet, _query_index: u32) {
        log::info!("Ignored Pop Debug Group")
    }

    fn begin_pipeline_statistics_query(&mut self, _query_set: &wgpu::custom::DispatchQuerySet, _query_index: u32) {
        log::error!("Called begin_pipeline_statistics_query");
        panic!()
    }

    fn end_pipeline_statistics_query(&mut self) {
        log::error!("Called end_pipeline_statistics_query");
        panic!()
    }

    fn dispatch_workgroups(&mut self, x: u32, y: u32, z: u32) {
        unsafe { webgpu_cp_dispatch_workgroups(self.ident, x, y, z) }
    }

    fn dispatch_workgroups_indirect(&mut self, indirect_buffer: &wgpu::custom::DispatchBuffer, indirect_offset: wgpu::BufferAddress) {
        unsafe {
            webgpu_cp_dispatch_workgroups_indirect(
                self.ident,
                indirect_buffer.as_custom::<crate::EdgeGpuBuffer>().unwrap().ident,
                indirect_offset,
            )
        }
    }

    fn end(&mut self) {
        todo!()
    }
}
