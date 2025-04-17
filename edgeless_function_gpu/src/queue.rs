// SPDX-FileCopyrightText: © 2021 The gfx-rs developers
// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
//
// See lib.rs for more information about the initial source.

extern "C" {
    fn webgpu_queue_write_buffer(queue_id: u64, buffer_id: u64, offset: u64, data_ptr: *const u8, data_len: u64);
    fn webgpu_queue_submit(queue_id: u64, command_buffers_ptr: *const u8, command_buffers_len: u64) -> u64;
    fn webgpu_queue_get_timestamp_period(queue_id: u64) -> f32;
}

#[derive(Debug)]
pub struct EdgeGpuQueue {
    pub(crate) ident: u64,
}

impl Drop for EdgeGpuQueue {
    fn drop(&mut self) {
        unsafe {
            crate::webgpu_drop(self.ident);
        }
    }
}

impl wgpu::custom::QueueInterface for EdgeGpuQueue {
    fn write_buffer(&self, buffer: &wgpu::custom::DispatchBuffer, offset: wgpu::BufferAddress, data: &[u8]) {
        let buffer_id = buffer.as_custom_opt().unwrap().downcast::<crate::EdgeGpuBuffer>().unwrap().ident;
        unsafe { webgpu_queue_write_buffer(self.ident, buffer_id, offset, data.as_ptr(), data.len() as u64) };
    }

    fn create_staging_buffer(&self, _size: wgpu::BufferSize) -> Option<wgpu::custom::DispatchQueueWriteBuffer> {
        log::error!("create_staging_buffer");
        panic!();
    }

    fn validate_write_buffer(&self, _buffer: &wgpu::custom::DispatchBuffer, _offset: wgpu::BufferAddress, _size: wgpu::BufferSize) -> Option<()> {
        log::error!("validate_write_buffer");
        panic!();
    }

    fn write_staging_buffer(
        &self,
        _buffer: &wgpu::custom::DispatchBuffer,
        _offset: wgpu::BufferAddress,
        _staging_buffer: &wgpu::custom::DispatchQueueWriteBuffer,
    ) {
        log::error!("write_staging_buffer");
        panic!();
    }

    fn write_texture(
        &self,
        _texture: wgpu::TexelCopyTextureInfo<'_>,
        _data: &[u8],
        _data_layout: wgpu::TexelCopyBufferLayout,
        _size: wgpu::Extent3d,
    ) {
        log::error!("write_texture");
        panic!();
    }

    fn submit(&self, command_buffers: &mut dyn Iterator<Item = wgpu::custom::DispatchCommandBuffer>) -> u64 {
        let command_buffers: Vec<_> = command_buffers.collect();

        let command_buffers: Vec<_> = command_buffers
            .iter()
            .map(|c| c.as_custom_opt().unwrap().downcast::<crate::EdgeGpuCommandBuffer>().unwrap().ident)
            .collect();
        let command_buffers = serde_json::to_vec(&command_buffers).unwrap();

        unsafe { webgpu_queue_submit(self.ident, command_buffers.as_ptr(), command_buffers.len() as u64) }
    }

    fn get_timestamp_period(&self) -> f32 {
        unsafe { webgpu_queue_get_timestamp_period(self.ident) }
    }

    fn on_submitted_work_done(&self, _callback: wgpu::custom::BoxSubmittedWorkDoneCallback) {
        log::info!("Tried to Set Work Done Callback");
    }
}
