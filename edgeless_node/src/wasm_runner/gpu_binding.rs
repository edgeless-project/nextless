// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

pub struct GPUWrapper {
    instances: std::collections::HashMap<u64, WGPUInstance>,
    adapters: std::collections::HashMap<u64, WGPUAdapter>,
    devices: std::collections::HashMap<u64, WGPUDevice>,
    queues: std::collections::HashMap<u64, WGPUQueue>,
    shaders: std::collections::HashMap<u64, WGPUShaderModule>,
    bind_group_layouts: std::collections::HashMap<u64, WGPUBindGroupLayout>,
    bind_groups: std::collections::HashMap<u64, WGPUBindGroup>,
    pipeline_layouts: std::collections::HashMap<u64, WGPUPipelineLayout>,
    compute_pipelines: std::collections::HashMap<u64, WGPUComputePipeline>,
    buffers: std::collections::HashMap<u64, WGPUBuffer>,
    command_encoders: std::collections::HashMap<u64, WGPUCommandEncoder>,
    command_buffers: std::collections::HashMap<u64, WGPUCommandBuffer>,
    compute_passes: std::collections::HashMap<u64, u64>,
    next_id: u64,
}

#[derive(Debug)]
pub enum WGPUError {
    NotFound,
    AdapterCreation,
    DeviceCreation,
    BindGroupLayoutCreation,
    BindGroupCreation,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct BindGroupEntrySerializer {
    binding: u32,
    resource: BindGroupEntrySerializerResource,
}

#[derive(serde::Serialize, serde::Deserialize)]
enum BindGroupEntrySerializerResource {
    Buffer(u64),
    Buffers(Vec<u64>),
}

pub(crate) struct BindGroupEntryContainer<'a> {
    binding: u32,
    resource: BindGroupEntryResourceContainer<'a>,
}

enum BindGroupEntryResourceContainer<'a> {
    Buffer(wgpu::BufferBinding<'a>),
    Buffers(Vec<wgpu::BufferBinding<'a>>),
}

impl GPUWrapper {
    pub(crate) fn new() -> Self {
        Self {
            instances: std::collections::HashMap::new(),
            adapters: std::collections::HashMap::new(),
            next_id: 1,
            devices: std::collections::HashMap::new(),
            queues: std::collections::HashMap::new(),
            shaders: std::collections::HashMap::new(),
            bind_group_layouts: std::collections::HashMap::new(),
            bind_groups: std::collections::HashMap::new(),
            pipeline_layouts: std::collections::HashMap::new(),
            compute_pipelines: std::collections::HashMap::new(),
            buffers: std::collections::HashMap::new(),
            command_encoders: std::collections::HashMap::new(),
            command_buffers: std::collections::HashMap::new(),
            compute_passes: std::collections::HashMap::new(),
        }
    }

    pub(crate) fn new_instance(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.instances.insert(
            id,
            WGPUInstance {
                instance: wgpu::Instance::new(&wgpu::InstanceDescriptor::default()),
                id,
            },
        );
        id
    }

    pub(crate) async fn new_adapter(&mut self, instance_id: u64) -> Result<u64, WGPUError> {
        let id = self.next_id;
        self.next_id += 1;

        let adapter = self
            .instances
            .get_mut(&instance_id)
            .ok_or(WGPUError::AdapterCreation)?
            .instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .map_err(|_| WGPUError::AdapterCreation)?;

        log::info!("{:?}", adapter.get_info());

        self.adapters.insert(id, WGPUAdapter { id, adapter });
        Ok(id)
    }

    pub(crate) async fn new_device(&mut self, adapter_id: u64) -> Result<(u64, u64), WGPUError> {
        let device_id = self.next_id;
        self.next_id += 1;
        let queue_id = self.next_id;
        self.next_id += 1;

        let (device, queue) = self
            .adapters
            .get(&adapter_id)
            .unwrap()
            .adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .unwrap();

        self.devices.insert(device_id, WGPUDevice { device, id: device_id });
        self.queues.insert(queue_id, WGPUQueue { queue, id: queue_id });
        Ok((device_id, queue_id))
    }

    pub(crate) fn new_shader_module(&mut self, device_id: u64, code: &[u8]) -> Result<u64, WGPUError> {
        let id = self.next_id;
        self.next_id += 1;
        self.shaders.insert(
            id,
            WGPUShaderModule {
                id,
                shader_module: self
                    .devices
                    .get(&device_id)
                    .ok_or(WGPUError::BindGroupLayoutCreation)?
                    .device
                    .create_shader_module(wgpu::ShaderModuleDescriptor {
                        label: None,
                        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Owned(String::from_utf8(code.to_vec()).unwrap())),
                    }),
            },
        );
        Ok(id)
    }

    pub(crate) fn new_bind_group_layout(&mut self, device_id: u64, layout_entries: &[wgpu::BindGroupLayoutEntry]) -> Result<u64, WGPUError> {
        let id = self.next_id;
        self.next_id += 1;
        self.bind_group_layouts.insert(
            id,
            WGPUBindGroupLayout {
                id,
                bind_group_layout: self
                    .devices
                    .get(&device_id)
                    .ok_or(WGPUError::BindGroupLayoutCreation)?
                    .device
                    .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                        label: None,
                        entries: layout_entries,
                    }),
            },
        );
        Ok(id)
    }

    pub(crate) fn new_bind_group(&mut self, device_id: u64, layout_id: u64, entries: &[BindGroupEntrySerializer]) -> Result<u64, WGPUError> {
        let layout = self.bind_group_layouts.get(&layout_id).ok_or(WGPUError::BindGroupCreation)?;

        let entries: Vec<_> = entries
            .iter()
            .map(|e| BindGroupEntryContainer {
                binding: e.binding,
                resource: match &e.resource {
                    BindGroupEntrySerializerResource::Buffer(buffer_id) => {
                        BindGroupEntryResourceContainer::Buffer(self.buffers.get(buffer_id).unwrap().buffer.as_entire_buffer_binding())
                    }
                    BindGroupEntrySerializerResource::Buffers(items) => BindGroupEntryResourceContainer::Buffers(
                        items
                            .iter()
                            .map(|i| self.buffers.get(i).unwrap().buffer.as_entire_buffer_binding())
                            .collect::<Vec<_>>(),
                    ),
                },
            })
            .collect();

        let entries: Vec<_> = entries
            .iter()
            .map(|e| wgpu::BindGroupEntry {
                binding: e.binding,
                resource: match &e.resource {
                    BindGroupEntryResourceContainer::Buffer(buffer_binding) => wgpu::BindingResource::Buffer(buffer_binding.clone()),
                    BindGroupEntryResourceContainer::Buffers(buffer_bindings) => wgpu::BindingResource::BufferArray(buffer_bindings.as_slice()),
                },
            })
            .collect();

        let id = self.next_id;
        self.next_id += 1;
        self.bind_groups.insert(
            id,
            WGPUBindGroup {
                id,
                bind_group: self
                    .devices
                    .get(&device_id)
                    .ok_or(WGPUError::BindGroupLayoutCreation)?
                    .device
                    .create_bind_group(&wgpu::BindGroupDescriptor {
                        label: None,
                        entries: entries.as_slice(),
                        layout: &layout.bind_group_layout,
                    }),
            },
        );
        Ok(id)
    }

    pub(crate) fn new_pipeline_layout(
        &mut self,
        device_id: u64,
        bind_group_layouts: &[u64],
        push_constant_ranges: &[wgpu::PushConstantRange],
    ) -> Result<u64, WGPUError> {
        let bind_group_layouts: Vec<_> = bind_group_layouts
            .iter()
            .map(|id| &self.bind_group_layouts.get(id).unwrap().bind_group_layout)
            .collect();

        let id = self.next_id;
        self.next_id += 1;
        self.pipeline_layouts.insert(
            id,
            WGPUPipelineLayout {
                id,
                pippeline_layout: self
                    .devices
                    .get(&device_id)
                    .ok_or(WGPUError::BindGroupLayoutCreation)?
                    .device
                    .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: None,
                        bind_group_layouts: &bind_group_layouts[..],
                        push_constant_ranges,
                    }),
            },
        );
        Ok(id)
    }

    pub(crate) fn new_compute_pipeline(
        &mut self,
        device_id: u64,
        layout: Option<u64>,
        shader_module: u64,
        entry_point: Option<&str>,
    ) -> Result<u64, WGPUError> {
        let layout = layout.map(|layout_id| &self.pipeline_layouts.get(&layout_id).unwrap().pippeline_layout);

        let shader_module = self.shaders.get(&shader_module).ok_or(WGPUError::NotFound)?;

        let descriptor = wgpu::ComputePipelineDescriptor {
            label: None,
            layout,
            module: &shader_module.shader_module,
            entry_point,
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        };

        let id = self.next_id;
        self.next_id += 1;
        self.compute_pipelines.insert(
            id,
            WGPUComputePipeline {
                id,
                pipeline: self
                    .devices
                    .get(&device_id)
                    .ok_or(WGPUError::BindGroupLayoutCreation)?
                    .device
                    .create_compute_pipeline(&descriptor),
            },
        );
        Ok(id)
    }

    pub(crate) fn new_buffer(&mut self, device_id: u64, desc: &wgpu::BufferDescriptor) -> Result<u64, WGPUError> {
        let device = self.devices.get(&device_id).ok_or(WGPUError::NotFound)?;
        let id = self.next_id;
        self.next_id += 1;

        let buffer = device.device.create_buffer(desc);
        self.buffers.insert(id, WGPUBuffer { buffer, id });
        Ok(id)
    }

    pub(crate) fn new_command_encoder(&mut self, device_id: u64) -> Result<u64, WGPUError> {
        let device = self.devices.get(&device_id).ok_or(WGPUError::NotFound)?;
        let id = self.next_id;
        self.next_id += 1;

        let command_encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        self.command_encoders.insert(
            id,
            WGPUCommandEncoder {
                command_encoder,
                id,
                compute_passes: std::collections::HashMap::new(),
            },
        );
        Ok(id)
    }

    pub(crate) fn device_poll(&mut self, device_id: u64, maintain: wgpu::PollType) -> Result<wgpu::PollStatus, wgpu::PollError> {
        let device = self.devices.get(&device_id).unwrap();

        device.device.poll(maintain)
    }

    pub(crate) fn queue_write_buffer(&mut self, queue_id: u64, buffer_id: u64, offset: u64, data: &[u8]) -> Result<(), WGPUError> {
        let queue = self.queues.get(&queue_id).unwrap();
        let buffer = self.buffers.get(&buffer_id).unwrap();

        queue.queue.write_buffer(&buffer.buffer, offset, data);
        Ok(())
    }

    pub(crate) fn queue_submit(&mut self, queue_id: u64, command_buffer_ids: &[u64]) -> Result<u64, WGPUError> {
        let queue = self.queues.get(&queue_id).unwrap();
        let command_buffers = command_buffer_ids
            .iter()
            .map(|cb_id| self.command_buffers.remove(cb_id).unwrap().command_buffer);

        queue.queue.submit(command_buffers);
        Ok(0)
    }

    pub(crate) fn buffer_map_async(&mut self, buffer_id: u64, mode: wgpu::MapMode, start: u64, end: u64) -> Result<(), WGPUError> {
        let buffer = self.buffers.get(&buffer_id).unwrap();

        buffer.buffer.map_async(mode, std::ops::Range { start, end }, |_| {
            log::info!("Unimplemented Callback Called");
        });
        Ok(())
    }

    pub(crate) fn mapped_range_read(&mut self, buffer_id: u64, sub_range_start: u64, sub_range_end: u64) -> Result<Vec<u8>, WGPUError> {
        let buffer = self.buffers.get(&buffer_id).unwrap();

        Ok(buffer
            .buffer
            .get_mapped_range(std::ops::Range {
                start: sub_range_start,
                end: sub_range_end,
            })
            .to_vec())
    }

    pub(crate) fn mapped_range_write(&mut self, buffer_id: u64, sub_range_start: u64, sub_range_end: u64, data: &[u8]) -> Result<(), WGPUError> {
        let buffer = self.buffers.get_mut(&buffer_id).unwrap();

        buffer
            .buffer
            .get_mapped_range_mut(std::ops::Range {
                start: sub_range_start,
                end: sub_range_end,
            })
            .copy_from_slice(data);

        Ok(())
    }

    pub(crate) fn ce_copy_buffer_to_buffer(
        &mut self,
        ce_id: u64,
        source_buffer_id: u64,
        source_offset: u64,
        dest_buffer_id: u64,
        dest_offset: u64,
        copy_size: u64,
    ) -> Result<(), WGPUError> {
        let command_encoder = self.command_encoders.get_mut(&ce_id).unwrap();
        let source_buffer = self.buffers.get(&source_buffer_id).unwrap();
        let dest_buffer = self.buffers.get(&dest_buffer_id).unwrap();

        command_encoder
            .command_encoder
            .copy_buffer_to_buffer(&source_buffer.buffer, source_offset, &dest_buffer.buffer, dest_offset, copy_size);
        Ok(())
    }

    pub(crate) fn ce_begin_compute_pass(&mut self, ce_id: u64) -> Result<u64, WGPUError> {
        let id = self.next_id;
        self.next_id += 1;

        self.command_encoders.get_mut(&ce_id).unwrap().begin_compute_pass(id);
        self.compute_passes.insert(id, ce_id);

        Ok(id)
    }

    pub(crate) fn ce_finish(&mut self, ce_id: u64) -> Result<u64, WGPUError> {
        let command_encoder = self.command_encoders.remove(&ce_id).unwrap();

        let id = self.next_id;
        self.next_id += 1;
        let command_buffer = command_encoder.command_encoder.finish();
        self.command_buffers.insert(id, WGPUCommandBuffer { command_buffer, id });
        Ok(id)
    }

    pub(crate) fn cp_set_pipeline(&mut self, compute_pass_id: u64, pipeline_id: u64) -> Result<(), WGPUError> {
        let relevant_ce_id = self.compute_passes.get(&compute_pass_id).unwrap();
        let compute_pass = self
            .command_encoders
            .get_mut(relevant_ce_id)
            .unwrap()
            .compute_passes
            .get_mut(&compute_pass_id)
            .unwrap();
        let pipeline = self.compute_pipelines.get(&pipeline_id).unwrap();

        compute_pass.compute_pass.set_pipeline(&pipeline.pipeline);
        Ok(())
    }

    pub(crate) fn cp_set_bind_group(
        &mut self,
        compute_pass_id: u64,
        index: u32,
        bind_group_id: Option<u64>,
        offsets: &[wgpu::DynamicOffset],
    ) -> Result<(), WGPUError> {
        let relevant_ce_id = self.compute_passes.get(&compute_pass_id).unwrap();
        let compute_pass = self
            .command_encoders
            .get_mut(relevant_ce_id)
            .unwrap()
            .compute_passes
            .get_mut(&compute_pass_id)
            .unwrap();
        let bind_group = bind_group_id.map(|id| &self.bind_groups.get(&id).unwrap().bind_group);

        compute_pass.compute_pass.set_bind_group(index, bind_group, offsets);
        Ok(())
    }

    pub(crate) fn cp_dispatch_workgroups(&mut self, compute_pass_id: u64, x: u32, y: u32, z: u32) -> Result<(), WGPUError> {
        let relevant_ce_id = self.compute_passes.get(&compute_pass_id).unwrap();
        let compute_pass = self
            .command_encoders
            .get_mut(relevant_ce_id)
            .unwrap()
            .compute_passes
            .get_mut(&compute_pass_id)
            .unwrap();

        compute_pass.compute_pass.dispatch_workgroups(x, y, z);
        Ok(())
    }

    pub(crate) fn cp_dispatch_workgroups_indirect(
        &mut self,
        compute_pass_id: u64,
        indirect_buffer_id: u64,
        indirect_buffer_offset: u64,
    ) -> Result<(), WGPUError> {
        let relevant_ce_id = self.compute_passes.get(&compute_pass_id).unwrap();
        let compute_pass = self
            .command_encoders
            .get_mut(relevant_ce_id)
            .unwrap()
            .compute_passes
            .get_mut(&compute_pass_id)
            .unwrap();
        let buffer = self.buffers.get(&indirect_buffer_id).unwrap();

        compute_pass
            .compute_pass
            .dispatch_workgroups_indirect(&buffer.buffer, indirect_buffer_offset);
        Ok(())
    }

    pub(crate) fn cp_set_push_constants(&mut self, compute_pass_id: u64, offset: u32, data: &[u8]) -> Result<(), WGPUError> {
        let relevant_ce_id = self.compute_passes.get(&compute_pass_id).unwrap();
        let compute_pass = self
            .command_encoders
            .get_mut(relevant_ce_id)
            .unwrap()
            .compute_passes
            .get_mut(&compute_pass_id)
            .unwrap();

        compute_pass.compute_pass.set_push_constants(offset, data);
        Ok(())
    }

    pub(crate) fn cp_drop(&mut self, compute_pass_id: u64) -> Result<(), WGPUError> {
        let relevant_ce_id = self.compute_passes.get(&compute_pass_id).unwrap();
        self.command_encoders
            .get_mut(relevant_ce_id)
            .unwrap()
            .compute_passes
            .remove(&compute_pass_id)
            .unwrap();
        self.compute_passes.remove(&compute_pass_id);

        Ok(())
    }

    pub(crate) fn buffer_unmap(&mut self, buffer_id: u64) -> Result<(), WGPUError> {
        let buffer = self.buffers.get(&buffer_id).unwrap();
        buffer.buffer.unmap();
        Ok(())
    }

    pub(crate) fn get_mut(&mut self, id: u64) -> Result<&mut WGPUInstance, WGPUError> {
        self.instances.get_mut(&id).ok_or(WGPUError::NotFound)
    }
}

pub struct WGPUInstance {
    instance: wgpu::Instance,
    #[allow(unused)]
    id: u64,
}

impl WGPUInstance {
    pub fn poll_all(&mut self, forced: bool) -> bool {
        self.instance.poll_all(forced)
    }

    pub fn wsgl_language_features(&self) -> u32 {
        self.instance.wgsl_language_features().bits()
    }
}

pub struct WGPUAdapter {
    adapter: wgpu::Adapter,
    #[allow(unused)]
    id: u64,
}

impl WGPUAdapter {
    // async fn device_create(&self) -> Result<(u64, u64), WGPUError> {}
}

pub struct WGPUDevice {
    device: wgpu::Device,
    #[allow(unused)]
    id: u64,
}

pub struct WGPUQueue {
    queue: wgpu::Queue,
    #[allow(unused)]
    id: u64,
}

pub struct WGPUShaderModule {
    shader_module: wgpu::ShaderModule,
    #[allow(unused)]
    id: u64,
}

pub struct WGPUBindGroupLayout {
    bind_group_layout: wgpu::BindGroupLayout,
    #[allow(unused)]
    id: u64,
}

pub struct WGPUBindGroup {
    bind_group: wgpu::BindGroup,
    #[allow(unused)]
    id: u64,
}

pub struct WGPUPipelineLayout {
    pippeline_layout: wgpu::PipelineLayout,
    #[allow(unused)]
    id: u64,
}

pub struct WGPUComputePipeline {
    pipeline: wgpu::ComputePipeline,
    #[allow(unused)]
    id: u64,
}

pub struct WGPUBuffer {
    buffer: wgpu::Buffer,
    #[allow(unused)]
    id: u64,
}

pub struct WGPUCommandEncoder {
    command_encoder: wgpu::CommandEncoder,
    compute_passes: std::collections::HashMap<u64, WGPUComputePass>,
    #[allow(unused)]
    id: u64,
}

impl WGPUCommandEncoder {
    fn begin_compute_pass(&mut self, id: u64) {
        let pass = self.command_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        self.compute_passes.insert(
            id,
            WGPUComputePass {
                compute_pass: pass.forget_lifetime(),
                id,
            },
        );
    }
}

pub struct WGPUCommandBuffer {
    command_buffer: wgpu::CommandBuffer,
    #[allow(unused)]
    id: u64,
}

pub struct WGPUComputePass {
    compute_pass: wgpu::ComputePass<'static>,
    #[allow(unused)]
    id: u64,
}
