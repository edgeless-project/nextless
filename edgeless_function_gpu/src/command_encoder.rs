// SPDX-FileCopyrightText: © 2021 The gfx-rs developers
// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
//
// See lib.rs for more information about the initial source.

extern "C" {
    fn webgpu_ce_copy_buffer_to_buffer(ce_id: u64, source_id: u64, source_offset: u64, destination_id: u64, destination_offset: u64, copy_size: u64);
    fn webgpu_ce_begin_compute_pass(command_encoded_id: u64) -> u64;
    fn webgpu_ce_finish(command_encoded_id: u64) -> u64;
}

#[derive(Debug)]
pub struct EdgeGpuCommandEncoder {
    pub(crate) ident: u64,
}

impl wgpu::custom::CommandEncoderInterface for EdgeGpuCommandEncoder {
    fn copy_buffer_to_buffer(
        &self,
        source: &wgpu::custom::DispatchBuffer,
        source_offset: wgpu::BufferAddress,
        destination: &wgpu::custom::DispatchBuffer,
        destination_offset: wgpu::BufferAddress,
        copy_size: wgpu::BufferAddress,
    ) {
        unsafe {
            webgpu_ce_copy_buffer_to_buffer(
                self.ident,
                source.as_custom().downcast::<crate::EdgeGpuBuffer>().ident,
                source_offset,
                destination.as_custom().downcast::<crate::EdgeGpuBuffer>().ident,
                destination_offset,
                copy_size,
            )
        }
    }

    fn copy_buffer_to_texture(
        &self,
        _source: wgpu::TexelCopyBufferInfo<'_>,
        _destination: wgpu::TexelCopyTextureInfo<'_>,
        _copy_size: wgpu::Extent3d,
    ) {
        log::error!("Tried to copy buffer to texture");
        panic!()
    }

    fn copy_texture_to_buffer(
        &self,
        _source: wgpu::TexelCopyTextureInfo<'_>,
        _destination: wgpu::TexelCopyBufferInfo<'_>,
        _copy_size: wgpu::Extent3d,
    ) {
        log::error!("Tried to copy texture to buffer");
        panic!()
    }

    fn copy_texture_to_texture(
        &self,
        _source: wgpu::TexelCopyTextureInfo<'_>,
        _destination: wgpu::TexelCopyTextureInfo<'_>,
        _copy_size: wgpu::Extent3d,
    ) {
        log::error!("Tried to copy texture to texture");
        panic!()
    }

    fn begin_compute_pass(&self, _desc: &wgpu::ComputePassDescriptor<'_>) -> wgpu::custom::DispatchComputePass {
        let ident = unsafe { webgpu_ce_begin_compute_pass(self.ident) };
        wgpu::custom::DispatchComputePass::custom(crate::EdgeGpuComputePass { ident })
    }

    fn begin_render_pass(&self, _desc: &wgpu::RenderPassDescriptor<'_>) -> wgpu::custom::DispatchRenderPass {
        log::error!("Tried to start render pass");
        panic!()
    }

    fn finish(&mut self) -> wgpu::custom::DispatchCommandBuffer {
        let ident = unsafe { webgpu_ce_finish(self.ident) };
        wgpu::custom::DispatchCommandBuffer::custom(crate::EdgeGpuCommandBuffer { ident })
    }

    fn clear_texture(&self, _texture: &wgpu::custom::DispatchTexture, _subresource_range: &wgpu::ImageSubresourceRange) {
        log::error!("Tried to clear texture");
        panic!()
    }

    fn clear_buffer(&self, _buffer: &wgpu::custom::DispatchBuffer, _offset: wgpu::BufferAddress, _size: Option<wgpu::BufferAddress>) {
        log::info!("Tried to clear buffer");
        // panic!()
    }

    fn insert_debug_marker(&self, _label: &str) {
        log::error!("Tried to insert debug marker");
        panic!()
    }

    fn push_debug_group(&self, _label: &str) {
        log::error!("Tried to push debug group");
        panic!()
    }

    fn pop_debug_group(&self) {
        log::error!("Tried to pop debug group");
        panic!()
    }

    fn write_timestamp(&self, _query_set: &wgpu::custom::DispatchQuerySet, _query_index: u32) {
        log::error!("Tried to write query timestamp");
        panic!()
    }

    fn resolve_query_set(
        &self,
        _query_set: &wgpu::custom::DispatchQuerySet,
        _first_query: u32,
        _query_count: u32,
        _destination: &wgpu::custom::DispatchBuffer,
        _destination_offset: wgpu::BufferAddress,
    ) {
        log::error!("Tried to resolve query set");
        panic!()
    }

    fn mark_acceleration_structures_built<'a>(
        &self,
        _blas: &mut dyn Iterator<Item = &'a wgpu::Blas>,
        _tlas: &mut dyn Iterator<Item = &'a wgpu::Tlas>,
    ) {
        log::error!("Tried to mark_acceleration_structures_built");
        panic!()
    }

    fn build_acceleration_structures_unsafe_tlas<'a>(
        &self,
        _blas: &mut dyn Iterator<Item = &'a wgpu::BlasBuildEntry<'a>>,
        _tlas: &mut dyn Iterator<Item = &'a wgpu::TlasBuildEntry<'a>>,
    ) {
        log::error!("Tried to build_acceleration_structures_unsafe_tlas");
        panic!()
    }

    fn build_acceleration_structures<'a>(
        &self,
        _blas: &mut dyn Iterator<Item = &'a wgpu::BlasBuildEntry<'a>>,
        _tlas: &mut dyn Iterator<Item = &'a wgpu::TlasPackage>,
    ) {
        log::error!("Tried to build_acceleration_structures");
        panic!()
    }

    fn transition_resources<'a>(
        &mut self,
        _buffer_transitions: &mut dyn Iterator<Item = wgpu::wgt::BufferTransition<&'a wgpu::custom::DispatchBuffer>>,
        _texture_transitions: &mut dyn Iterator<Item = wgpu::wgt::TextureTransition<&'a wgpu::custom::DispatchTexture>>,
    ) {
        log::error!("Tried to transition_resources");
        panic!()
    }
}
