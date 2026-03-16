// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

use wasmtime::AsContextMut;

/// Binds the WASM component's imports to the function's GuestAPIHost.
pub struct GuestAPI {
    pub host: crate::base_runtime::guest_api::GuestAPIHost,
    pub wgpu_wrapper: super::gpu_binding::GPUWrapper,
}

pub async fn telemetry_log(
    mut caller: wasmtime::Caller<'_, GuestAPI>,
    level: i32,
    target_ptr: i32,
    target_len: i32,
    msg_ptr: i32,
    msg_len: i32,
) -> wasmtime::Result<()> {
    let mem = get_memory(&mut caller)?;
    let target = super::helpers::load_string_from_vm(&mut caller.as_context_mut(), &mem, target_ptr, target_len)?;
    let msg = super::helpers::load_string_from_vm(&mut caller.as_context_mut(), &mem, msg_ptr, msg_len)?;

    caller
        .data_mut()
        .host
        .telemetry_log(super::helpers::level_from_i32(level), &target, &msg)
        .await;
    Ok(())
}

pub async fn cast_raw(
    mut caller: wasmtime::Caller<'_, GuestAPI>,
    instance_node_id_ptr: i32,
    instance_component_id_ptr: i32,
    port_ptr: i32,
    port_len: i32,
    payload_ptr: i32,
    payload_len: i32,
) -> wasmtime::Result<()> {
    let mem = get_memory(&mut caller)?;
    let node_id = mem.data_mut(&mut caller)[instance_node_id_ptr as usize..(instance_node_id_ptr as usize) + 16_usize].to_vec();
    let component_id = mem.data_mut(&mut caller)[instance_component_id_ptr as usize..(instance_component_id_ptr as usize) + 16_usize].to_vec();
    let instance_id = edgeless_api::function_instance::InstanceId {
        node_id: uuid::Uuid::from_bytes(node_id.try_into().map_err(|_| wasmtime::Error::msg("uuid error"))?),
        function_id: uuid::Uuid::from_bytes(component_id.try_into().map_err(|_| wasmtime::Error::msg("uuid error"))?),
    };

    let port = super::helpers::load_string_from_vm(&mut caller.as_context_mut(), &mem, port_ptr, port_len)?;
    let payload = super::helpers::load_from_vm(&mut caller.as_context_mut(), &mem, payload_ptr, payload_len)?;

    caller
        .data_mut()
        .host
        .cast_raw(instance_id, edgeless_api::function_instance::PortId(port), &payload)
        .await
        .map_err(|_| wasmtime::Error::msg("string error"))?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn call_raw(
    mut caller: wasmtime::Caller<'_, GuestAPI>,
    instance_node_id_ptr: i32,
    instance_component_id_ptr: i32,
    port_ptr: i32,
    port_len: i32,
    payload_ptr: i32,
    payload_len: i32,
    out_ptr_ptr: i32,
    out_len_ptr: i32,
) -> wasmtime::Result<i32> {
    let mem = get_memory(&mut caller)?;
    let alloc = get_alloc(&mut caller)?;
    let node_id = mem.data_mut(&mut caller)[instance_node_id_ptr as usize..(instance_node_id_ptr as usize) + 16_usize].to_vec();
    let component_id = mem.data_mut(&mut caller)[instance_component_id_ptr as usize..(instance_component_id_ptr as usize) + 16_usize].to_vec();
    let instance_id = edgeless_api::function_instance::InstanceId {
        node_id: uuid::Uuid::from_bytes(node_id.try_into().map_err(|_| wasmtime::Error::msg("uuid error"))?),
        function_id: uuid::Uuid::from_bytes(component_id.try_into().map_err(|_| wasmtime::Error::msg("uuid error"))?),
    };

    let port = super::helpers::load_string_from_vm(&mut caller.as_context_mut(), &mem, port_ptr, port_len)?;
    let payload = super::helpers::load_from_vm(&mut caller.as_context_mut(), &mem, payload_ptr, payload_len)?;

    let call_ret = caller
        .data_mut()
        .host
        .call_raw(instance_id, edgeless_api::function_instance::PortId(port), &payload)
        .await
        .map_err(|_| wasmtime::Error::msg("call error"))?;
    match call_ret {
        crate::dataplane::core::CallRet::NoReply => Ok(0),
        crate::dataplane::core::CallRet::Reply(data) => {
            let len = data.len();

            let data_ptr = super::helpers::copy_to_vm(&mut caller.as_context_mut(), &mem, &alloc, &data).await?;
            super::helpers::copy_to_vm_ptr(&mut caller.as_context_mut(), &mem, out_ptr_ptr, &data_ptr.to_le_bytes())?;
            super::helpers::copy_to_vm_ptr(&mut caller.as_context_mut(), &mem, out_len_ptr, &len.to_le_bytes())?;

            Ok(1)
        }
        crate::dataplane::core::CallRet::Err => Ok(2),
    }
}

pub async fn cast(
    mut caller: wasmtime::Caller<'_, GuestAPI>,
    target_ptr: i32,
    target_len: i32,
    payload_ptr: i32,
    payload_len: i32,
) -> wasmtime::Result<()> {
    let mem = get_memory(&mut caller)?;

    let target = super::helpers::load_string_from_vm(&mut caller.as_context_mut(), &mem, target_ptr, target_len)?;
    let payload = super::helpers::load_from_vm(&mut caller.as_context_mut(), &mem, payload_ptr, payload_len)?;

    match caller.data_mut().host.cast_alias(&target, &payload).await {
        Ok(_) => {}
        Err(_) => {
            // We ignore casts to unknown targets.
            tracing::warn!("Cast to unknown target: {target}");
        }
    };

    Ok(())
}

pub async fn call(
    mut caller: wasmtime::Caller<'_, GuestAPI>,
    target_ptr: i32,
    target_len: i32,
    payload_ptr: i32,
    payload_len: i32,
    out_ptr_ptr: i32,
    out_len_ptr: i32,
) -> wasmtime::Result<i32> {
    let mem = get_memory(&mut caller)?;
    let alloc = get_alloc(&mut caller)?;

    let target = super::helpers::load_string_from_vm(&mut caller.as_context_mut(), &mem, target_ptr, target_len)?;
    let payload = super::helpers::load_from_vm(&mut caller.as_context_mut(), &mem, payload_ptr, payload_len)?;

    let call_ret = caller
        .data_mut()
        .host
        .call_alias(&target, &payload)
        .await
        .map_err(|_| wasmtime::Error::msg("call error"))?;
    match call_ret {
        crate::dataplane::core::CallRet::NoReply => Ok(0),
        crate::dataplane::core::CallRet::Reply(data) => {
            let len = data.len();

            let data_ptr = super::helpers::copy_to_vm(&mut caller.as_context_mut(), &mem, &alloc, &data).await?;
            super::helpers::copy_to_vm_ptr(&mut caller.as_context_mut(), &mem, out_ptr_ptr, &data_ptr.to_le_bytes())?;
            super::helpers::copy_to_vm_ptr(&mut caller.as_context_mut(), &mem, out_len_ptr, &len.to_le_bytes())?;

            Ok(1)
        }
        crate::dataplane::core::CallRet::Err => Ok(2),
    }
}

pub async fn delayed_cast(
    mut caller: wasmtime::Caller<'_, GuestAPI>,
    delay_ms: i64,
    target_ptr: i32,
    target_len: i32,
    payload_ptr: i32,
    payload_len: i32,
) -> wasmtime::Result<()> {
    let mem = get_memory(&mut caller)?;
    let target = super::helpers::load_string_from_vm(&mut caller.as_context_mut(), &mem, target_ptr, target_len)?;
    let payload = super::helpers::load_from_vm(&mut caller.as_context_mut(), &mem, payload_ptr, payload_len)?;

    caller
        .data_mut()
        .host
        .delayed_cast(delay_ms as u64, &target, &payload)
        .await
        .map_err(|_| wasmtime::Error::msg("call error"))?;
    Ok(())
}

pub async fn sync(mut caller: wasmtime::Caller<'_, GuestAPI>, state_ptr: i32, state_len: i32) -> wasmtime::Result<()> {
    let mem = get_memory(&mut caller)?;
    let state = super::helpers::load_string_from_vm(&mut caller.as_context_mut(), &mem, state_ptr, state_len)?;

    caller
        .data_mut()
        .host
        .sync(&state)
        .await
        .map_err(|_| wasmtime::Error::msg("sync error"))?;
    Ok(())
}

pub async fn slf(mut caller: wasmtime::Caller<'_, GuestAPI>, out_node_id_ptr: i32, out_component_id_ptr: i32) -> wasmtime::Result<()> {
    let mem = get_memory(&mut caller)?;

    let id = caller.data_mut().host.slf().await;

    super::helpers::copy_to_vm_ptr(&mut caller.as_context_mut(), &mem, out_node_id_ptr, id.node_id.as_bytes())?;
    super::helpers::copy_to_vm_ptr(&mut caller.as_context_mut(), &mem, out_component_id_ptr, id.function_id.as_bytes())?;

    Ok(())
}

pub async fn wgpu_instance_new(mut caller: wasmtime::Caller<'_, GuestAPI>) -> wasmtime::Result<u64> {
    tracing::debug!("New Instance");
    Ok(caller.data_mut().wgpu_wrapper.new_instance())
}

pub async fn wgpu_instance_poll_all(mut caller: wasmtime::Caller<'_, GuestAPI>, instance_id: u64, forced: u32) -> wasmtime::Result<u32> {
    match caller
        .data_mut()
        .wgpu_wrapper
        .get_mut(instance_id)
        .map_err(|_| wasmtime::Error::msg("Unknown WGPU Instance"))?
        .poll_all(forced > 0)
    {
        true => Ok(1),
        false => Ok(0),
    }
}

pub async fn webgpu_instance_adapter_create(mut caller: wasmtime::Caller<'_, GuestAPI>, instance_id: u64) -> wasmtime::Result<u64> {
    tracing::debug!("Create Adapter");
    let ret = caller.data_mut().wgpu_wrapper.new_adapter(instance_id).await;

    ret.map_err(|_| wasmtime::Error::msg("WGPU Adapter Create Failure"))
}

pub async fn webgpu_instance_wgsl_language_features(mut caller: wasmtime::Caller<'_, GuestAPI>, instance_id: u64) -> wasmtime::Result<u32> {
    Ok(caller
        .data_mut()
        .wgpu_wrapper
        .get_mut(instance_id)
        .map_err(|_| wasmtime::Error::msg("Unknown WGPU Instance"))?
        .wsgl_language_features())
}

pub async fn webgpu_adapter_device_create(
    mut caller: wasmtime::Caller<'_, GuestAPI>,
    adapter_id: u64,
    out_device_id: i32,
    out_queue_id: i32,
) -> wasmtime::Result<u32> {
    let mem = get_memory(&mut caller)?;

    tracing::debug!("Create Device");

    let (device, queue) = caller
        .data_mut()
        .wgpu_wrapper
        .new_device(adapter_id)
        .await
        .map_err(|_| wasmtime::Error::msg("New Device Error"))?;

    tracing::debug!("System was able to create a Device.");

    super::helpers::copy_to_vm_ptr(&mut caller.as_context_mut(), &mem, out_device_id, &device.to_le_bytes())?;
    super::helpers::copy_to_vm_ptr(&mut caller.as_context_mut(), &mem, out_queue_id, &queue.to_le_bytes())?;
    Ok(1)
}

pub fn webgpu_device_create_shader_module(
    mut caller: wasmtime::Caller<'_, GuestAPI>,
    device_id: u64,
    wgsl: i32,
    wgsl_len: u64,
) -> wasmtime::Result<u64> {
    let mem = get_memory(&mut caller)?;
    let code = mem.data_mut(caller.as_context_mut())[wgsl as usize..(wgsl as usize) + wgsl_len as usize].to_vec();
    caller
        .data_mut()
        .wgpu_wrapper
        .new_shader_module(device_id, code.as_slice())
        .map_err(|_| wasmtime::Error::msg("Create Shader Error"))
}

pub fn webgpu_device_create_bind_group_layout(
    mut caller: wasmtime::Caller<'_, GuestAPI>,
    device_id: u64,
    entry_ptr: i32,
    entry_len: u64,
) -> wasmtime::Result<u64> {
    let mem = get_memory(&mut caller)?;
    let entries = mem.data_mut(caller.as_context_mut())[entry_ptr as usize..(entry_ptr as usize) + entry_len as usize].to_vec();
    let entries: Vec<wgpu::BindGroupLayoutEntry> = serde_json::from_slice(&entries).unwrap();

    caller
        .data_mut()
        .wgpu_wrapper
        .new_bind_group_layout(device_id, entries.as_slice())
        .map_err(|_| wasmtime::Error::msg("Create Bind Group Layout Error"))
}

pub fn webgpu_device_create_bind_group(
    mut caller: wasmtime::Caller<'_, GuestAPI>,
    device_id: u64,
    layout_id: u64,
    entry_ptr: i32,
    entry_len: u64,
) -> wasmtime::Result<u64> {
    let mem = get_memory(&mut caller)?;
    let entries = mem.data_mut(caller.as_context_mut())[entry_ptr as usize..(entry_ptr as usize) + entry_len as usize].to_vec();
    let entries: Vec<super::gpu_binding::BindGroupEntrySerializer> = serde_json::from_slice(&entries).unwrap();

    caller
        .data_mut()
        .wgpu_wrapper
        .new_bind_group(device_id, layout_id, entries.as_slice())
        .map_err(|_| wasmtime::Error::msg("Create  Bind Group Error"))
}

pub fn webgpu_device_create_dipatch_pipeline_layout(
    mut caller: wasmtime::Caller<'_, GuestAPI>,
    device_id: u64,
    layouts_ptr: i32,
    layouts_len: u64,
    push_constants_ptr: i32,
    push_constants_len: u64,
) -> wasmtime::Result<u64> {
    let mem = get_memory(&mut caller)?;

    let layouts = mem.data_mut(caller.as_context_mut())[layouts_ptr as usize..(layouts_ptr as usize) + layouts_len as usize].to_vec();
    let layouts: Vec<u64> = serde_json::from_slice(&layouts).unwrap();

    let push_constants =
        mem.data_mut(caller.as_context_mut())[push_constants_ptr as usize..(push_constants_ptr as usize) + push_constants_len as usize].to_vec();
    let push_constants: Vec<wgpu::PushConstantRange> = serde_json::from_slice(&push_constants).unwrap();

    caller
        .data_mut()
        .wgpu_wrapper
        .new_pipeline_layout(device_id, layouts.as_slice(), push_constants.as_slice())
        .map_err(|_| wasmtime::Error::msg("Create Pipeline Layout Error"))
}

pub fn webgpu_device_create_compute_pipeline(
    mut caller: wasmtime::Caller<'_, GuestAPI>,
    device_id: u64,
    layout_id: u64,
    module_id: u64,
    entry_point_ptr: i32,
    entry_point_len: u64,
) -> wasmtime::Result<u64> {
    let mem = get_memory(&mut caller)?;

    let entry_point = if entry_point_len == 0 {
        None
    } else {
        let e = mem.data_mut(caller.as_context_mut())[entry_point_ptr as usize..(entry_point_ptr as usize) + entry_point_len as usize].to_vec();
        Some(String::from_utf8(e).unwrap())
    };
    let entry_point = entry_point.as_deref();

    let layout_id = if layout_id == 0 { None } else { Some(layout_id) };

    caller
        .data_mut()
        .wgpu_wrapper
        .new_compute_pipeline(device_id, layout_id, module_id, entry_point)
        .map_err(|_| wasmtime::Error::msg("Create Compute Pipeline Error"))
}

pub fn webgpu_device_create_buffer(
    mut caller: wasmtime::Caller<'_, GuestAPI>,
    device_id: u64,
    buffer_desc_ptr: i32,
    buffer_desc_len: u64,
) -> wasmtime::Result<u64> {
    let mem = get_memory(&mut caller)?;

    let buffer_desc = mem.data_mut(caller.as_context_mut())[buffer_desc_ptr as usize..(buffer_desc_ptr as usize) + buffer_desc_len as usize].to_vec();
    let buffer_desc: wgpu::BufferDescriptor = serde_json::from_slice(&buffer_desc).unwrap();

    caller
        .data_mut()
        .wgpu_wrapper
        .new_buffer(device_id, &buffer_desc)
        .map_err(|_| wasmtime::Error::msg("Create Buffer Error"))
}

pub fn webgpu_device_create_command_encoder(mut caller: wasmtime::Caller<'_, GuestAPI>, device_id: u64) -> wasmtime::Result<u64> {
    caller
        .data_mut()
        .wgpu_wrapper
        .new_command_encoder(device_id)
        .map_err(|_| wasmtime::Error::msg("Create Command Encoder Error"))
}

pub fn webgpu_device_poll(
    mut caller: wasmtime::Caller<'_, GuestAPI>,
    device_id: u64,
    poll_type: u64,
    #[allow(unused)] submission_index: u64,
) -> wasmtime::Result<u64> {
    let maintain = match poll_type {
        2 => wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        },
        3 => wgpu::PollType::Poll,
        _ => {
            tracing::warn!("Actor attempted to use unsupported poll type: {poll_type}");
            return Err(wasmtime::Error::msg("Bad Poll Type"));
        }
    };

    let status: Result<wgpu::PollStatus, wgpu::PollError> = caller.data_mut().wgpu_wrapper.device_poll(device_id, maintain);
    // .map_err(|e| wasmtime::Error::msg("Device Poll Error Error"));

    let res = if let Ok(status) = status {
        match status {
            wgpu::PollStatus::QueueEmpty => 1,
            wgpu::PollStatus::WaitSucceeded => 2,
            wgpu::PollStatus::Poll => 3,
        }
    } else {
        0
    };
    Ok(res)
}

pub fn webgpu_queue_write_buffer(
    mut caller: wasmtime::Caller<'_, GuestAPI>,
    queue_id: u64,
    buffer_id: u64,
    offset: u64,
    data_ptr: i32,
    data_len: u64,
) -> wasmtime::Result<()> {
    let mem = get_memory(&mut caller)?;

    let data = mem.data_mut(caller.as_context_mut())[data_ptr as usize..(data_ptr as usize) + data_len as usize].to_vec();

    caller
        .data_mut()
        .wgpu_wrapper
        .queue_write_buffer(queue_id, buffer_id, offset, data.as_slice())
        .map_err(|_| wasmtime::Error::msg("Queue Write Error"))?;
    Ok(())
}

pub fn webgpu_queue_submit(
    mut caller: wasmtime::Caller<'_, GuestAPI>,
    queue_id: u64,
    command_buffers_ptr: i32,
    command_buffers_len: u64,
) -> wasmtime::Result<u64> {
    let mem = get_memory(&mut caller)?;

    let command_buffers =
        mem.data_mut(caller.as_context_mut())[command_buffers_ptr as usize..(command_buffers_ptr as usize) + command_buffers_len as usize].to_vec();
    let command_buffers: Vec<u64> = serde_json::from_slice(&command_buffers).unwrap();

    caller
        .data_mut()
        .wgpu_wrapper
        .queue_submit(queue_id, command_buffers.as_slice())
        .map_err(|_| wasmtime::Error::msg("Queue Submit Error"))
}

pub fn webgpu_queue_get_timestamp_period(mut _caller: wasmtime::Caller<'_, GuestAPI>, _queue_id: u64) -> wasmtime::Result<f32> {
    tracing::warn!("Unimplemented handler 'webgpu_queue_get_timestamp_period' called");
    Err(wasmtime::Error::msg("Unimplemented webgpu_queue_get_timestamp_period called"))
}

pub fn webgpu_buffer_map_async(mut caller: wasmtime::Caller<'_, GuestAPI>, buffer_id: u64, mode: u64, start: u64, end: u64) -> wasmtime::Result<()> {
    let mode = match mode {
        1 => wgpu::MapMode::Read,
        2 => wgpu::MapMode::Write,
        _ => return Err(wasmtime::Error::msg("Bad MapMode")),
    };

    caller
        .data_mut()
        .wgpu_wrapper
        .buffer_map_async(buffer_id, mode, start, end)
        .map_err(|_| wasmtime::Error::msg("Buffer Map Async Error"))?;
    Ok(())
}

pub fn webgpu_buffer_mapped_range_read(
    mut caller: wasmtime::Caller<'_, GuestAPI>,
    buffer_id: u64,
    sub_range_start: u64,
    sub_range_end: u64,
    data_ptr: i32,
) -> wasmtime::Result<()> {
    let mem = get_memory(&mut caller)?;
    let data = caller
        .data_mut()
        .wgpu_wrapper
        .mapped_range_read(buffer_id, sub_range_start, sub_range_end)
        .map_err(|_| wasmtime::Error::msg("Buffer Get Mapped Range Error"))?;
    super::helpers::copy_to_vm_ptr(&mut caller.as_context_mut(), &mem, data_ptr, data.as_slice())?;
    Ok(())
}

pub fn webgpu_buffer_mapped_range_write(
    mut caller: wasmtime::Caller<'_, GuestAPI>,
    buffer_id: u64,
    sub_range_start: u64,
    sub_range_end: u64,
    data_ptr: i32,
) -> wasmtime::Result<()> {
    let mem = get_memory(&mut caller)?;

    let data =
        &mem.data_mut(caller.as_context_mut())[data_ptr as usize..(data_ptr as usize) + sub_range_end as usize - sub_range_start as usize].to_vec();

    caller
        .data_mut()
        .wgpu_wrapper
        .mapped_range_write(buffer_id, sub_range_start, sub_range_end, data.as_slice())
        .map_err(|_| wasmtime::Error::msg("Buffer Get Mapped Range Error"))?;
    Ok(())
}

pub fn webgpu_ce_copy_buffer_to_buffer(
    mut caller: wasmtime::Caller<'_, GuestAPI>,
    ce_id: u64,
    source_buffer_id: u64,
    source_offset: u64,
    dest_buffer_id: u64,
    dest_offset: u64,
    copy_size: u64,
) -> wasmtime::Result<()> {
    caller
        .data_mut()
        .wgpu_wrapper
        .ce_copy_buffer_to_buffer(ce_id, source_buffer_id, source_offset, dest_buffer_id, dest_offset, copy_size)
        .map_err(|_| wasmtime::Error::msg("Copy Buffer To Buffer Error"))?;
    Ok(())
}

pub fn webgpu_ce_begin_compute_pass(mut caller: wasmtime::Caller<'_, GuestAPI>, ce_id: u64) -> wasmtime::Result<u64> {
    caller
        .data_mut()
        .wgpu_wrapper
        .ce_begin_compute_pass(ce_id)
        .map_err(|_| wasmtime::Error::msg("CE Compute Pass Begin Error"))
}

pub fn webgpu_ce_finish(mut caller: wasmtime::Caller<'_, GuestAPI>, ce_id: u64) -> wasmtime::Result<u64> {
    caller
        .data_mut()
        .wgpu_wrapper
        .ce_finish(ce_id)
        .map_err(|_| wasmtime::Error::msg("CE Finish Error"))
}

pub fn webgpu_cp_set_pipeline(mut caller: wasmtime::Caller<'_, GuestAPI>, compute_pass_id: u64, pipeline_id: u64) -> wasmtime::Result<()> {
    caller
        .data_mut()
        .wgpu_wrapper
        .cp_set_pipeline(compute_pass_id, pipeline_id)
        .map_err(|_| wasmtime::Error::msg("CP Set Pipeline Error"))?;
    Ok(())
}

pub fn webgpu_cp_set_bind_group(
    mut caller: wasmtime::Caller<'_, GuestAPI>,
    compute_pass_id: u64,
    index: u32,
    bind_group_id: u64,
    offsets_ptr: i32,
    offsets_len: u64,
) -> wasmtime::Result<()> {
    let mem = get_memory(&mut caller)?;

    let offsets = mem.data_mut(caller.as_context_mut())[offsets_ptr as usize..(offsets_ptr as usize) + offsets_len as usize].to_vec();
    let offsets: Vec<wgpu::DynamicOffset> = serde_json::from_slice(&offsets).unwrap();

    let bind_group_id = if bind_group_id == 0 { None } else { Some(bind_group_id) };

    caller
        .data_mut()
        .wgpu_wrapper
        .cp_set_bind_group(compute_pass_id, index, bind_group_id, offsets.as_slice())
        .map_err(|_| wasmtime::Error::msg("CP Set BindGroup Error"))?;
    Ok(())
}

pub fn webgpu_cp_dispatch_workgroups(
    mut caller: wasmtime::Caller<'_, GuestAPI>,
    compute_pass_id: u64,
    x: u32,
    y: u32,
    z: u32,
) -> wasmtime::Result<()> {
    caller
        .data_mut()
        .wgpu_wrapper
        .cp_dispatch_workgroups(compute_pass_id, x, y, z)
        .map_err(|_| wasmtime::Error::msg("CP Dispatch Error"))
}

pub fn webgpu_cp_dispatch_workgroups_indirect(
    mut caller: wasmtime::Caller<'_, GuestAPI>,
    compute_pass_id: u64,
    indirect_buffer_id: u64,
    indirect_offset: u64,
) -> wasmtime::Result<()> {
    caller
        .data_mut()
        .wgpu_wrapper
        .cp_dispatch_workgroups_indirect(compute_pass_id, indirect_buffer_id, indirect_offset)
        .map_err(|_| wasmtime::Error::msg("CP Dispatch Indirect Error"))
}

pub fn webgpu_cp_set_push_constants(
    mut caller: wasmtime::Caller<'_, GuestAPI>,
    compute_pass_id: u64,
    offset: u32,
    data_ptr: i32,
    data_len: u64,
) -> wasmtime::Result<()> {
    let mem = get_memory(&mut caller)?;

    let data = mem.data_mut(caller.as_context_mut())[data_ptr as usize..(data_ptr as usize) + data_len as usize].to_vec();

    caller
        .data_mut()
        .wgpu_wrapper
        .cp_set_push_constants(compute_pass_id, offset, data.as_slice())
        .map_err(|_| wasmtime::Error::msg("CP Set Push Constants Error"))?;
    Ok(())
}

pub fn webgpu_cp_drop(mut caller: wasmtime::Caller<'_, GuestAPI>, compute_pass_id: u64) -> wasmtime::Result<()> {
    caller
        .data_mut()
        .wgpu_wrapper
        .cp_drop(compute_pass_id)
        .map_err(|_| wasmtime::Error::msg("CP Drop Error"))?;
    Ok(())
}

pub fn webgpu_buffer_unmap(mut caller: wasmtime::Caller<'_, GuestAPI>, buffer_id: u64) -> wasmtime::Result<()> {
    caller
        .data_mut()
        .wgpu_wrapper
        .buffer_unmap(buffer_id)
        .map_err(|_| wasmtime::Error::msg("Buffer Unmap Error"))?;
    Ok(())
}

pub fn webgpu_compute_pipeline_get_bind_group_layout(
    mut caller: wasmtime::Caller<'_, GuestAPI>,
    compute_pipeline_id: u64,
    index: u32,
) -> wasmtime::Result<u64> {
    caller
        .data_mut()
        .wgpu_wrapper
        .compute_pipeline_get_bind_group_layout(compute_pipeline_id, index)
        .map_err(|_| wasmtime::Error::msg("Compute Pipeline Get Bind Group Error"))
}

pub fn webgpu_drop(mut caller: wasmtime::Caller<'_, GuestAPI>, resource_id: u64) -> wasmtime::Result<()> {
    caller
        .data_mut()
        .wgpu_wrapper
        .drop_resource(resource_id)
        .map_err(|_| wasmtime::Error::msg("Resource Drop Error"))?;
    Ok(())
}

pub(crate) fn get_memory(caller: &mut wasmtime::Caller<'_, super::guest_api_binding::GuestAPI>) -> wasmtime::Result<wasmtime::Memory> {
    caller
        .get_export("memory")
        .ok_or(wasmtime::Error::msg("memory error"))?
        .into_memory()
        .ok_or(wasmtime::Error::msg("memory error"))
}

pub(crate) fn get_alloc(caller: &mut wasmtime::Caller<'_, super::guest_api_binding::GuestAPI>) -> wasmtime::Result<wasmtime::TypedFunc<i32, i32>> {
    caller
        .get_export("edgeless_mem_alloc")
        .ok_or(wasmtime::Error::msg("alloc error"))?
        .into_func()
        .ok_or(wasmtime::Error::msg("alloc error"))?
        .typed::<i32, i32>(&caller)
        .map_err(|_| wasmtime::Error::msg("alloc error"))
}
