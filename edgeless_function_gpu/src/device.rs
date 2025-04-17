// SPDX-FileCopyrightText: © 2021 The gfx-rs developers
// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
//
// See lib.rs for more information about the initial source.

extern "C" {
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
}

#[derive(Debug)]
pub struct EdgeGpuDevice {
    pub(crate) ident: u64,
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

impl Drop for EdgeGpuDevice {
    fn drop(&mut self) {
        unsafe {
            crate::webgpu_drop(self.ident);
        }
    }
}

impl wgpu::custom::DeviceInterface for EdgeGpuDevice {
    fn features(&self) -> wgpu::Features {
        wgpu::Features::all_webgpu_mask()
    }

    fn limits(&self) -> wgpu::Limits {
        wgpu::Limits::default()
    }

    fn create_shader_module(
        &self,
        desc: wgpu::ShaderModuleDescriptor<'_>,
        _shader_bound_checks: wgpu::ShaderRuntimeChecks,
    ) -> wgpu::custom::DispatchShaderModule {
        match desc.source {
            wgpu::ShaderSource::Wgsl(ref code) => {
                let shader_id = unsafe { webgpu_device_create_shader_module(self.ident, code.as_ptr(), code.len() as u64) };
                wgpu::custom::DispatchShaderModule::custom(crate::EdgeGpuShaderModule { ident: shader_id })
            }
            _ => {
                panic!("Invalid Shader")
            }
        }
    }

    unsafe fn create_shader_module_passthrough(&self, _desc: &wgpu::ShaderModuleDescriptorPassthrough<'_>) -> wgpu::custom::DispatchShaderModule {
        unreachable!("No XXX_SHADER_PASSTHROUGH feature enabled for this backend")
    }

    fn create_bind_group_layout(&self, desc: &wgpu::BindGroupLayoutDescriptor<'_>) -> wgpu::custom::DispatchBindGroupLayout {
        let entries = serde_json::to_vec(desc.entries).unwrap();

        let bind_group_ident = unsafe { webgpu_device_create_bind_group_layout(self.ident, entries.as_ptr(), entries.len() as u64) };

        wgpu::custom::DispatchBindGroupLayout::custom(crate::EdgeGpuBindGroupLayout { ident: bind_group_ident })
    }

    fn create_bind_group(&self, desc: &wgpu::BindGroupDescriptor<'_>) -> wgpu::custom::DispatchBindGroup {
        let entries: Vec<BindGroupEntrySerializer> = desc.entries.iter().map(BindGroupEntrySerializer::from).collect();

        let entries = serde_json::to_vec(&entries).unwrap();

        let bind_group_ident = unsafe {
            webgpu_device_create_bind_group(
                self.ident,
                desc.layout.as_custom::<crate::EdgeGpuBindGroupLayout>().unwrap().ident,
                entries.as_ptr(),
                entries.len() as u64,
            )
        };

        wgpu::custom::DispatchBindGroup::custom(crate::EdgeGpuBindGroup { ident: bind_group_ident })
    }

    fn create_pipeline_layout(&self, desc: &wgpu::PipelineLayoutDescriptor<'_>) -> wgpu::custom::DispatchPipelineLayout {
        let bind_group_layouts = serde_json::to_vec(
            &desc
                .bind_group_layouts
                .iter()
                .map(|l| l.as_custom::<crate::EdgeGpuBindGroupLayout>().unwrap().ident)
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

        wgpu::custom::DispatchPipelineLayout::custom(crate::EdgeGpuPipelineLayout { ident })
    }

    fn create_render_pipeline(&self, _desc: &wgpu::RenderPipelineDescriptor<'_>) -> wgpu::custom::DispatchRenderPipeline {
        log::error!("Tried using create_render_pipeline");
        panic!();
    }

    fn create_compute_pipeline(&self, desc: &wgpu::ComputePipelineDescriptor<'_>) -> wgpu::custom::DispatchComputePipeline {
        let pipeline_layout_id = match desc.layout {
            Some(l) => l.as_custom::<crate::EdgeGpuPipelineLayout>().unwrap().ident,
            None => 0,
        };

        let module_id = desc.module.as_custom::<crate::EdgeGpuShaderModule>().unwrap().ident;

        let entry_point = desc.entry_point.unwrap_or_default();

        let ident = unsafe {
            webgpu_device_create_compute_pipeline(self.ident, pipeline_layout_id, module_id, entry_point.as_ptr(), entry_point.len() as u64)
        };

        wgpu::custom::DispatchComputePipeline::custom(crate::EdgeGpuComputePipeline { ident })
    }

    unsafe fn create_pipeline_cache(&self, _desc: &wgpu::PipelineCacheDescriptor<'_>) -> wgpu::custom::DispatchPipelineCache {
        log::error!("Tried using create_pipeline_cache");
        panic!();
    }

    fn create_buffer(&self, desc: &wgpu::BufferDescriptor<'_>) -> wgpu::custom::DispatchBuffer {
        let ser_desc = serde_json::to_vec(desc).unwrap();

        let ident = unsafe { webgpu_device_create_buffer(self.ident, ser_desc.as_ptr(), ser_desc.len() as u64) };

        wgpu::custom::DispatchBuffer::custom(crate::EdgeGpuBuffer { ident })
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
        wgpu::custom::DispatchCommandEncoder::custom(crate::EdgeGpuCommandEncoder { ident })
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

    fn pop_error_scope(&self) -> std::pin::Pin<Box<dyn wgpu::custom::PopErrorScopeFuture>> {
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
        #[allow(unused_mut)]
        let mut submission_id: u64 = 0;
        let poll_type = match maintain {
            wgpu::wgt::PollType::WaitForSubmissionIndex(_i) => 1,
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

impl From<&wgpu::BindGroupEntry<'_>> for BindGroupEntrySerializer {
    fn from(value: &wgpu::BindGroupEntry) -> Self {
        Self {
            binding: value.binding,
            resource: match &value.resource {
                wgpu::BindingResource::Buffer(buffer_binding) => {
                    BindGroupEntrySerializerResource::Buffer(buffer_binding.buffer.as_custom::<crate::EdgeGpuBuffer>().unwrap().ident)
                }
                wgpu::BindingResource::BufferArray(buffer_bindings) => BindGroupEntrySerializerResource::Buffers(
                    buffer_bindings
                        .iter()
                        .map(|b| b.buffer.as_custom::<crate::EdgeGpuBuffer>().unwrap().ident)
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
