// SPDX-FileCopyrightText: © 2021 The gfx-rs developers
// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
//
//
// Based on https://github.com/gfx-rs/wgpu/blob/trunk/examples/standalone/custom_backend/src/custom.rs (MIT/APACHE Licensed).

#![allow(dead_code)]
use std::pin::Pin;

use wgpu::custom::{
    AdapterInterface, BindGroupLayoutInterface, BufferMappedRangeInterface, DeviceInterface, DispatchAdapter, DispatchDevice, DispatchQueue,
    DispatchShaderModule, DispatchSurface, InstanceInterface, QueueInterface, RequestAdapterFuture, ShaderModuleInterface,
};

extern "C" {
    fn webgpu_instance_new() -> u64;
    fn webgpu_instance_poll_all(instance_id: u64, force_wait: u32) -> u32;
    fn webgpu_instance_adapter_create(instance_id: u64) -> u64;
    fn webgpu_instance_wgsl_language_features(instance_id: u64) -> u32;
    fn webgpu_adapter_device_create(instance_id: u64, out_device_id: *mut u64, out_queue_id: *mut u64) -> u32;
    fn webgpu_device_create_shader_module(instance_id: u64, wgsl: *const u8, wgsl_len: u64) -> u64;
    fn webgpu_device_create_bind_group_layout(instance_id: u64, entry_ptr: *const u8, entry_len: u64) -> u64;
    fn webgpu_device_create_bind_group(instance_id: u64, layout: u64, entry_ptr: *const u8, entry_len: u64) -> u64;
    fn webgpu_device_create_dipatch_pipeline_layout(
        device_id: u64,
        layouts_ptr: *const u8,
        layouts_len: u64,
        push_constants_ptr: *const u8,
        push_constants_len: u64,
    ) -> u64;
    fn webgpu_device_create_compute_pipeline(
        device_id: u64,
        pipeline_layout_id: u64,
        module_id: u64,
        entry_point_ptr: *const u8,
        entry_point_len: u64,
    ) -> u64;
    fn webgpu_device_create_buffer(device_id: u64, buffer_desc_ptr: *const u8, buffer_desc_len: u64) -> u64;
    fn webgpu_device_create_command_encoder(device_id: u64) -> u64;
    fn webgpu_device_poll(device_id: u64, poll_type: u64, submission_index: u64) -> u64;
    fn webgpu_queue_write_buffer(queue_id: u64, buffer_id: u64, offset: u64, data_ptr: *const u8, data_len: u64);
    fn webgpu_queue_submit(queue_id: u64, command_buffers_ptr: *const u8, command_buffers_len: u64) -> u64;
    fn webgpu_queue_get_timestamp_period(queue_id: u64) -> f32;
    fn webgpu_buffer_map_async(buffer_id: u64, mode: u64, start: u64, end: u64);
    fn webgpu_ce_copy_buffer_to_buffer(ce_id: u64, source_id: u64, source_offset: u64, destination_id: u64, destination_offset: u64, copy_size: u64);
    fn webgpu_ce_begin_compute_pass(command_encoded_id: u64) -> u64;
    fn webgpu_ce_finish(command_encoded_id: u64) -> u64;
    fn webgpu_cp_set_pipeline(compute_pass_id: u64, compute_pipeline_id: u64);
    fn webgpu_cp_set_bind_group(compute_pass_id: u64, index: u32, bind_group_id: u64, offsets_ptr: *const u8, offsets_len: u64);
    fn webgpu_cp_dispatch_workgroups(compute_pass_id: u64, x: u32, y: u32, z: u32);
    fn webgpu_cp_dispatch_workgroups_indirect(compute_pass_id: u64, indirect_buffer_id: u64, indirect_offset: u64);
    fn webgpu_cp_set_push_constants(compute_pass_id: u64, offset: u32, data_ptr: *const u8, data_len: u64);
    fn webgpu_buffer_mapped_range_read(buffer_id: u64, sub_range_start: u64, sub_range_end: u64, data: *mut u8);
    fn webgpu_buffer_mapped_range_write(buffer_id: u64, sub_range_start: u64, sub_range_end: u64, data: *const u8);
    fn webgpu_cp_drop(compute_pass_id: u64);
    fn webgpu_buffer_unmap(buffer_id: u64);
}

#[derive(Debug)]
pub struct CustomInstance {
    ident: u64,
}

#[derive(Debug)]
struct CustomAdapter {
    ident: u64,
}

#[derive(Debug)]
struct CustomDevice {
    ident: u64,
}

#[derive(Debug)]
struct CustomQueue {
    ident: u64,
}

#[derive(Debug)]
struct CustomShaderModule {
    ident: u64,
}

#[derive(Debug)]
struct CustomBindGroupLayout {
    ident: u64,
}

#[derive(Debug)]
struct CustomDispatchPipelineLayout {
    ident: u64,
}

#[derive(Debug)]
struct CustomBindGroup {
    ident: u64,
}

#[derive(Debug)]
struct CustomComputePipeline {
    ident: u64,
}

#[derive(Debug)]
struct CustomBuffer {
    ident: u64,
}

#[derive(Debug)]
struct CustomCommandEncoder {
    ident: u64,
}

#[derive(Debug)]
struct CustomCommandBuffer {
    ident: u64,
}

#[derive(Debug)]
struct CustomComputePass {
    ident: u64,
}

impl Drop for CustomComputePass {
    fn drop(&mut self) {
        unsafe {
            webgpu_cp_drop(self.ident);
        }
    }
}

#[derive(Debug)]
struct CustomMappedRange {
    buffer_id: u64,
    start: u64,
    end: u64,
    data: Vec<u8>,
    possibly_written: bool,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct BindGroupEntrySerializer {
    binding: u32,
    resource: BindGroupEntrySerializerResource,
}

#[derive(serde::Serialize, serde::Deserialize)]
enum BindGroupEntrySerializerResource {
    Buffer(u64),
    Buffers(Vec<u64>),
}

impl From<&wgpu::BindGroupEntry<'_>> for BindGroupEntrySerializer {
    fn from(value: &wgpu::BindGroupEntry) -> Self {
        Self {
            binding: value.binding,
            resource: match &value.resource {
                wgpu::BindingResource::Buffer(buffer_binding) => {
                    BindGroupEntrySerializerResource::Buffer(buffer_binding.buffer.inner.as_custom().downcast::<CustomBuffer>().ident)
                }
                wgpu::BindingResource::BufferArray(buffer_bindings) => BindGroupEntrySerializerResource::Buffers(
                    buffer_bindings
                        .iter()
                        .map(|b| b.buffer.inner.as_custom().downcast::<CustomBuffer>().ident)
                        .collect(),
                ),
                _ => {
                    log::error!("Unimplemented Binding Resource");
                    panic!();
                }
            },
        }
    }
}

impl BindGroupLayoutInterface for CustomBindGroupLayout {}

impl InstanceInterface for CustomInstance {
    fn new(desc: &wgpu::InstanceDescriptor) -> Self
    where
        Self: Sized,
    {
        let id = unsafe { webgpu_instance_new() };

        Self { ident: id }
    }

    unsafe fn create_surface(&self, _target: wgpu::SurfaceTargetUnsafe) -> Result<DispatchSurface, wgpu::CreateSurfaceError> {
        unimplemented!()
    }

    fn request_adapter(&self, _options: &wgpu::RequestAdapterOptions<'_, '_>) -> std::pin::Pin<Box<dyn RequestAdapterFuture>> {
        Box::pin(std::future::ready(Ok(DispatchAdapter::custom(CustomAdapter {
            ident: unsafe { webgpu_instance_adapter_create(self.ident) },
        }))))
    }

    fn poll_all_devices(&self, force_wait: bool) -> bool {
        unsafe {
            webgpu_instance_poll_all(
                self.ident,
                match force_wait {
                    true => 1,
                    false => 0,
                },
            ) > 0
        }
    }

    fn wgsl_language_features(&self) -> wgpu::WgslLanguageFeatures {
        let raw = unsafe { webgpu_instance_wgsl_language_features(self.ident) };
        wgpu::WgslLanguageFeatures::from_bits(raw).unwrap()
    }
}

impl AdapterInterface for CustomAdapter {
    fn request_device(&self, desc: &wgpu::DeviceDescriptor<'_>) -> Pin<Box<dyn wgpu::custom::RequestDeviceFuture>> {
        let mut device = 0u64;
        let mut queue = 0u64;

        assert!(unsafe { webgpu_adapter_device_create(self.ident, &mut device as *mut u64, &mut queue as *mut u64) > 0 });
        let device = CustomDevice { ident: device };
        let queue = CustomQueue { ident: queue };

        let res: Result<_, wgpu::RequestDeviceError> = Ok((DispatchDevice::custom(device), DispatchQueue::custom(queue)));
        Box::pin(std::future::ready(res))
    }

    fn is_surface_supported(&self, _surface: &DispatchSurface) -> bool {
        false
    }

    fn features(&self) -> wgpu::Features {
        wgpu::Features::all_webgpu_mask()
    }

    fn limits(&self) -> wgpu::Limits {
        wgpu::Limits::default()
    }

    fn downlevel_capabilities(&self) -> wgpu::DownlevelCapabilities {
        wgpu::DownlevelCapabilities::default()
    }

    fn get_info(&self) -> wgpu::AdapterInfo {
        log::info!("Called Adapter Info");
        wgpu::AdapterInfo {
            name: "Edgeless".to_string(),
            vendor: 0,
            device: 0,
            device_type: wgpu::DeviceType::VirtualGpu,
            driver: "".to_string(),
            driver_info: "".to_string(),
            backend: wgpu::Backend::BrowserWebGpu,
        }
    }

    fn get_texture_format_features(&self, format: wgpu::TextureFormat) -> wgpu::TextureFormatFeatures {
        format.guaranteed_format_features(AdapterInterface::features(self))
    }

    fn get_presentation_timestamp(&self) -> wgpu::PresentationTimestamp {
        wgpu::PresentationTimestamp::INVALID_TIMESTAMP
    }
}

impl DeviceInterface for CustomDevice {
    fn features(&self) -> wgpu::Features {
        wgpu::Features::all_webgpu_mask()
    }

    fn limits(&self) -> wgpu::Limits {
        wgpu::Limits::default()
    }

    fn create_shader_module(&self, desc: wgpu::ShaderModuleDescriptor<'_>, _shader_bound_checks: wgpu::ShaderRuntimeChecks) -> DispatchShaderModule {
        match desc.source {
            wgpu::ShaderSource::Wgsl(ref code) => {
                let shader_id = unsafe { webgpu_device_create_shader_module(self.ident, code.as_ptr(), code.len() as u64) };
                DispatchShaderModule::custom(CustomShaderModule { ident: shader_id })
            }
            _ => {
                panic!("Invalid Shader")
            }
        }
    }

    unsafe fn create_shader_module_passthrough(&self, _desc: &wgpu::ShaderModuleDescriptorPassthrough<'_>) -> DispatchShaderModule {
        unreachable!("No XXX_SHADER_PASSTHROUGH feature enabled for this backend")
    }

    fn create_bind_group_layout(&self, desc: &wgpu::BindGroupLayoutDescriptor<'_>) -> wgpu::custom::DispatchBindGroupLayout {
        let entries = serde_json::to_vec(desc.entries).unwrap();

        let bind_group_ident = unsafe { webgpu_device_create_bind_group_layout(self.ident, entries.as_ptr(), entries.len() as u64) };

        wgpu::custom::DispatchBindGroupLayout::custom(CustomBindGroupLayout { ident: bind_group_ident })
    }

    fn create_bind_group(&self, desc: &wgpu::BindGroupDescriptor<'_>) -> wgpu::custom::DispatchBindGroup {
        let entries: Vec<BindGroupEntrySerializer> = desc.entries.iter().map(|e| BindGroupEntrySerializer::from(e)).collect();

        let entries = serde_json::to_vec(&entries).unwrap();

        let bind_group_ident = unsafe {
            webgpu_device_create_bind_group(
                self.ident,
                desc.layout.inner.as_custom().downcast::<CustomBindGroupLayout>().ident,
                entries.as_ptr(),
                entries.len() as u64,
            )
        };

        wgpu::custom::DispatchBindGroup::custom(CustomBindGroup { ident: bind_group_ident })
    }

    fn create_pipeline_layout(&self, desc: &wgpu::PipelineLayoutDescriptor<'_>) -> wgpu::custom::DispatchPipelineLayout {
        let bind_group_layouts = serde_json::to_vec(
            &desc
                .bind_group_layouts
                .iter()
                .map(|l| l.inner.as_custom().downcast::<CustomBindGroupLayout>().ident)
                .collect::<Vec<u64>>(),
        )
        .unwrap();
        let push_constants = serde_json::to_vec(&desc.push_constant_ranges).unwrap();

        let ident = unsafe {
            webgpu_device_create_dipatch_pipeline_layout(
                self.ident,
                bind_group_layouts.as_ptr(),
                bind_group_layouts.len() as u64,
                push_constants.as_ptr(),
                push_constants.len() as u64,
            )
        };

        wgpu::custom::DispatchPipelineLayout::custom(CustomDispatchPipelineLayout { ident })
    }

    fn create_render_pipeline(&self, _desc: &wgpu::RenderPipelineDescriptor<'_>) -> wgpu::custom::DispatchRenderPipeline {
        log::error!("Tried using create_render_pipeline");
        panic!();
    }

    fn create_compute_pipeline(&self, desc: &wgpu::ComputePipelineDescriptor<'_>) -> wgpu::custom::DispatchComputePipeline {
        let pipeline_layout_id = match desc.layout {
            Some(l) => l.inner.as_custom().downcast::<CustomDispatchPipelineLayout>().ident,
            None => 0,
        };

        let module_id = desc.module.inner.as_custom().downcast::<CustomShaderModule>().ident;

        let entry_point = match desc.entry_point {
            Some(e) => e,
            None => "",
        };

        let ident = unsafe {
            webgpu_device_create_compute_pipeline(
                self.ident,
                pipeline_layout_id,
                module_id,
                entry_point.as_ptr(),
                entry_point.as_bytes().len() as u64,
            )
        };

        wgpu::custom::DispatchComputePipeline::custom(CustomComputePipeline { ident })
    }

    unsafe fn create_pipeline_cache(&self, _desc: &wgpu::PipelineCacheDescriptor<'_>) -> wgpu::custom::DispatchPipelineCache {
        log::error!("Tried using create_pipeline_cache");
        panic!();
    }

    fn create_buffer(&self, desc: &wgpu::BufferDescriptor<'_>) -> wgpu::custom::DispatchBuffer {
        let ser_desc = serde_json::to_vec(desc).unwrap();

        let ident = unsafe { webgpu_device_create_buffer(self.ident, ser_desc.as_ptr(), ser_desc.len() as u64) };

        wgpu::custom::DispatchBuffer::custom(CustomBuffer { ident })
    }

    fn create_texture(&self, _desc: &wgpu::TextureDescriptor<'_>) -> wgpu::custom::DispatchTexture {
        log::error!("Tried using create_texture");
        panic!();
    }

    fn create_blas(
        &self,
        _desc: &wgpu::CreateBlasDescriptor<'_>,
        _sizes: wgpu::BlasGeometrySizeDescriptors,
    ) -> (Option<u64>, wgpu::custom::DispatchBlas) {
        log::error!("Tried using create_blas");
        panic!();
    }

    fn create_tlas(&self, _desc: &wgpu::CreateTlasDescriptor<'_>) -> wgpu::custom::DispatchTlas {
        log::error!("Tried using create_tlas");
        panic!();
    }

    fn create_sampler(&self, _desc: &wgpu::SamplerDescriptor<'_>) -> wgpu::custom::DispatchSampler {
        log::error!("Tried using create_sampler");
        panic!();
    }

    fn create_query_set(&self, _desc: &wgpu::QuerySetDescriptor<'_>) -> wgpu::custom::DispatchQuerySet {
        log::error!("Tried using create_query_set");
        panic!();
    }

    fn create_command_encoder(&self, _desc: &wgpu::CommandEncoderDescriptor<'_>) -> wgpu::custom::DispatchCommandEncoder {
        let ident = unsafe { webgpu_device_create_command_encoder(self.ident) };
        wgpu::custom::DispatchCommandEncoder::custom(CustomCommandEncoder { ident })
    }

    fn create_render_bundle_encoder(&self, _desc: &wgpu::RenderBundleEncoderDescriptor<'_>) -> wgpu::custom::DispatchRenderBundleEncoder {
        log::error!("Tried using create_query_set");
        panic!();
    }

    fn set_device_lost_callback(&self, _device_lost_callback: wgpu::custom::BoxDeviceLostCallback) {
        log::info!("Set Device Lost Callback");
    }

    fn on_uncaptured_error(&self, _handler: Box<dyn wgpu::UncapturedErrorHandler>) {
        log::error!("on_uncaptured_error");
        panic!();
    }

    fn push_error_scope(&self, _filter: wgpu::ErrorFilter) {
        log::error!("push_error_scope");
        panic!();
    }

    fn pop_error_scope(&self) -> Pin<Box<dyn wgpu::custom::PopErrorScopeFuture>> {
        log::error!("pop_error_scope");
        panic!();
    }

    unsafe fn start_graphics_debugger_capture(&self) {
        log::error!("start_graphics_debugger_capture");
        panic!();
    }

    unsafe fn stop_graphics_debugger_capture(&self) {
        log::error!("stop_graphics_debugger_capture");
        panic!();
    }

    fn poll(&self, maintain: wgpu::PollType) -> Result<wgpu::PollStatus, wgpu::PollError> {
        let mut submission_id: u64 = 0;
        let poll_type = match maintain {
            wgpu::wgt::PollType::WaitForSubmissionIndex(i) => 1,
            wgpu::wgt::PollType::Wait => 2,
            wgpu::wgt::PollType::Poll => 3,
        };

        let status = unsafe { webgpu_device_poll(self.ident, poll_type, submission_id) };
        match status {
            1 => Ok(wgpu::PollStatus::QueueEmpty),
            2 => Ok(wgpu::PollStatus::WaitSucceeded),
            3 => Ok(wgpu::PollStatus::Poll),
            _ => Err(wgpu::PollError::Timeout),
        }
    }

    fn get_internal_counters(&self) -> wgpu::InternalCounters {
        wgpu::InternalCounters::default()
    }

    fn generate_allocator_report(&self) -> Option<wgpu::AllocatorReport> {
        None
    }

    fn destroy(&self) {
        log::info!("Tried to Destroy Device");
    }
}

impl ShaderModuleInterface for CustomShaderModule {
    fn get_compilation_info(&self) -> Pin<Box<dyn wgpu::custom::ShaderCompilationInfoFuture>> {
        Box::pin(std::future::ready(wgpu::CompilationInfo { messages: Vec::new() }))
    }
}

impl QueueInterface for CustomQueue {
    fn write_buffer(&self, buffer: &wgpu::custom::DispatchBuffer, offset: wgpu::BufferAddress, data: &[u8]) {
        let buffer_id = buffer.as_custom().downcast::<CustomBuffer>().ident;
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
        let command_buffers: Vec<_> = command_buffers.map(|c| c.as_custom().downcast::<CustomCommandBuffer>().ident).collect();
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

impl wgpu::custom::BufferInterface for CustomBuffer {
    fn map_async(&self, mode: wgpu::MapMode, range: std::ops::Range<wgpu::BufferAddress>, callback: wgpu::custom::BufferMapCallback) {
        let mode: u64 = match mode {
            wgpu::MapMode::Read => 1,
            wgpu::MapMode::Write => 2,
        };

        let start = range.start;
        let end = range.end;

        log::info!("Ignored Map Async Callback");

        unsafe { webgpu_buffer_map_async(self.ident, mode, start, end) }
    }

    fn get_mapped_range(&self, sub_range: std::ops::Range<wgpu::BufferAddress>) -> wgpu::custom::DispatchBufferMappedRange {
        // let ident = unsafe { webgpu_buffer_get_mapped_range(self.ident, sub_range.start, sub_range.end) };
        let data_len = sub_range.end - sub_range.start;
        let mut data = vec![0u8; data_len as usize];

        unsafe {
            webgpu_buffer_mapped_range_read(self.ident, sub_range.start, sub_range.end, data.as_mut_ptr());
        }

        wgpu::custom::DispatchBufferMappedRange::custom(CustomMappedRange {
            buffer_id: self.ident,
            start: sub_range.start,
            end: sub_range.end,
            data: data,
            possibly_written: false,
        })
    }

    fn unmap(&self) {
        unsafe {
            webgpu_buffer_unmap(self.ident);
        }
    }

    fn destroy(&self) {
        log::info!("Called Destroy");
    }
}

impl wgpu::custom::BindGroupInterface for CustomBindGroup {}

impl wgpu::custom::PipelineLayoutInterface for CustomDispatchPipelineLayout {}

impl wgpu::custom::ComputePipelineInterface for CustomComputePipeline {
    fn get_bind_group_layout(&self, index: u32) -> wgpu::custom::DispatchBindGroupLayout {
        log::error!("Tried to get Layout");
        panic!()
    }
}

impl wgpu::custom::CommandEncoderInterface for CustomCommandEncoder {
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
                source.as_custom().downcast::<CustomBuffer>().ident,
                source_offset,
                destination.as_custom().downcast::<CustomBuffer>().ident,
                destination_offset,
                copy_size,
            )
        }
    }

    fn copy_buffer_to_texture(&self, source: wgpu::TexelCopyBufferInfo<'_>, destination: wgpu::TexelCopyTextureInfo<'_>, copy_size: wgpu::Extent3d) {
        log::error!("Tried to copy buffer to texture");
        panic!()
    }

    fn copy_texture_to_buffer(&self, source: wgpu::TexelCopyTextureInfo<'_>, destination: wgpu::TexelCopyBufferInfo<'_>, copy_size: wgpu::Extent3d) {
        log::error!("Tried to copy texture to buffer");
        panic!()
    }

    fn copy_texture_to_texture(
        &self,
        source: wgpu::TexelCopyTextureInfo<'_>,
        destination: wgpu::TexelCopyTextureInfo<'_>,
        copy_size: wgpu::Extent3d,
    ) {
        log::error!("Tried to copy texture to texture");
        panic!()
    }

    fn begin_compute_pass(&self, desc: &wgpu::ComputePassDescriptor<'_>) -> wgpu::custom::DispatchComputePass {
        let ident = unsafe { webgpu_ce_begin_compute_pass(self.ident) };
        wgpu::custom::DispatchComputePass::custom(CustomComputePass { ident })
    }

    fn begin_render_pass(&self, desc: &wgpu::RenderPassDescriptor<'_>) -> wgpu::custom::DispatchRenderPass {
        log::error!("Tried to start render pass");
        panic!()
    }

    fn finish(&mut self) -> wgpu::custom::DispatchCommandBuffer {
        let ident = unsafe { webgpu_ce_finish(self.ident) };
        wgpu::custom::DispatchCommandBuffer::custom(CustomCommandBuffer { ident })
    }

    fn clear_texture(&self, texture: &wgpu::custom::DispatchTexture, subresource_range: &wgpu::ImageSubresourceRange) {
        log::error!("Tried to clear texture");
        panic!()
    }

    fn clear_buffer(&self, buffer: &wgpu::custom::DispatchBuffer, offset: wgpu::BufferAddress, size: Option<wgpu::BufferAddress>) {
        log::info!("Tried to clear buffer");
        // panic!()
    }

    fn insert_debug_marker(&self, label: &str) {
        log::error!("Tried to insert debug marker");
        panic!()
    }

    fn push_debug_group(&self, label: &str) {
        log::error!("Tried to push debug group");
        panic!()
    }

    fn pop_debug_group(&self) {
        log::error!("Tried to pop debug group");
        panic!()
    }

    fn write_timestamp(&self, query_set: &wgpu::custom::DispatchQuerySet, query_index: u32) {
        log::error!("Tried to write query timestamp");
        panic!()
    }

    fn resolve_query_set(
        &self,
        query_set: &wgpu::custom::DispatchQuerySet,
        first_query: u32,
        query_count: u32,
        destination: &wgpu::custom::DispatchBuffer,
        destination_offset: wgpu::BufferAddress,
    ) {
        log::error!("Tried to resolve query set");
        panic!()
    }

    fn mark_acceleration_structures_built<'a>(&self, blas: &mut dyn Iterator<Item = &'a wgpu::Blas>, tlas: &mut dyn Iterator<Item = &'a wgpu::Tlas>) {
        log::error!("Tried to mark_acceleration_structures_built");
        panic!()
    }

    fn build_acceleration_structures_unsafe_tlas<'a>(
        &self,
        blas: &mut dyn Iterator<Item = &'a wgpu::BlasBuildEntry<'a>>,
        tlas: &mut dyn Iterator<Item = &'a wgpu::TlasBuildEntry<'a>>,
    ) {
        log::error!("Tried to build_acceleration_structures_unsafe_tlas");
        panic!()
    }

    fn build_acceleration_structures<'a>(
        &self,
        blas: &mut dyn Iterator<Item = &'a wgpu::BlasBuildEntry<'a>>,
        tlas: &mut dyn Iterator<Item = &'a wgpu::TlasPackage>,
    ) {
        log::error!("Tried to build_acceleration_structures");
        panic!()
    }

    fn transition_resources<'a>(
        &mut self,
        buffer_transitions: &mut dyn Iterator<Item = wgpu::wgt::BufferTransition<&'a wgpu::custom::DispatchBuffer>>,
        texture_transitions: &mut dyn Iterator<Item = wgpu::wgt::TextureTransition<&'a wgpu::custom::DispatchTexture>>,
    ) {
        log::error!("Tried to transition_resources");
        panic!()
    }
}

impl wgpu::custom::CommandBufferInterface for CustomCommandBuffer {}

impl wgpu::custom::ComputePassInterface for CustomComputePass {
    fn set_pipeline(&mut self, pipeline: &wgpu::custom::DispatchComputePipeline) {
        unsafe { webgpu_cp_set_pipeline(self.ident, pipeline.as_custom().downcast::<CustomComputePipeline>().ident) }
    }

    fn set_bind_group(&mut self, index: u32, bind_group: Option<&wgpu::custom::DispatchBindGroup>, offsets: &[wgpu::DynamicOffset]) {
        let bind_group = match bind_group {
            Some(g) => g.as_custom().downcast::<CustomBindGroup>().ident,
            None => 0,
        };

        let offsets = serde_json::to_vec(offsets).unwrap();

        unsafe { webgpu_cp_set_bind_group(self.ident, index, bind_group, offsets.as_ptr(), offsets.len() as u64) }
    }

    fn set_push_constants(&mut self, offset: u32, data: &[u8]) {
        unsafe { webgpu_cp_set_push_constants(self.ident, offset, data.as_ptr(), data.len() as u64) }
    }

    fn insert_debug_marker(&mut self, label: &str) {
        log::info!("Ignored Debug Marker")
    }

    fn push_debug_group(&mut self, group_label: &str) {
        log::info!("Ignored Push Debug Group")
    }

    fn pop_debug_group(&mut self) {
        log::info!("Ignored Pop Debug Group")
    }

    fn write_timestamp(&mut self, query_set: &wgpu::custom::DispatchQuerySet, query_index: u32) {
        log::info!("Ignored Pop Debug Group")
    }

    fn begin_pipeline_statistics_query(&mut self, query_set: &wgpu::custom::DispatchQuerySet, query_index: u32) {
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
        unsafe { webgpu_cp_dispatch_workgroups_indirect(self.ident, indirect_buffer.as_custom().downcast::<CustomBuffer>().ident, indirect_offset) }
    }

    fn end(&mut self) {
        todo!()
    }
}

impl BufferMappedRangeInterface for CustomMappedRange {
    fn slice(&self) -> &[u8] {
        self.data.as_slice()
    }

    fn slice_mut(&mut self) -> &mut [u8] {
        self.possibly_written = true;
        self.data.as_mut_slice()
    }
}

impl Drop for CustomMappedRange {
    fn drop(&mut self) {
        if self.possibly_written {
            unsafe {
                webgpu_buffer_mapped_range_write(self.buffer_id, self.start, self.end, self.data.as_ptr());
            }
        }
    }
}
