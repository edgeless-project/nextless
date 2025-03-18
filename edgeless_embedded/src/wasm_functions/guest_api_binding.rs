// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

use wasmi::{AsContext, AsContextMut};

pub struct GuestAPI {
    pub host: super::guest_api::GuestAPIHost,
}

pub fn telemetry_log(
    mut caller: wasmi::Caller<'_, GuestAPI>,
    level: i32,
    target_ptr: i32,
    target_len: i32,
    msg_ptr: i32,
    msg_len: i32,
) -> Result<(), wasmi::Error> {
    let mem = get_memory(&mut caller)?;
    let ctx = caller.as_context();
    let target = super::helpers::load_str_from_vm(&ctx, &mem, target_ptr, target_len)?;
    let msg = super::helpers::load_str_from_vm(&ctx, &mem, msg_ptr, msg_len)?;
    let lvl = super::helpers::level_from_i32(level);

    embassy_futures::block_on(caller.data().host.telemetry_log(lvl, target, msg));
    Ok(())
}

pub fn cast_raw(
    mut caller: wasmi::Caller<'_, GuestAPI>,
    instance_node_id_ptr: i32,
    instance_component_id_ptr: i32,
    port_ptr: i32,
    port_len: i32,
    payload_ptr: i32,
    payload_len: i32,
) -> Result<(), wasmi::Error> {
    let mem = get_memory(&mut caller)?;
    let ctx = caller.as_context();

    let target_port = edgeless_api_core::port::Port(
        heapless::String::from_utf8(heapless::Vec::<u8, 32>::from_slice(super::helpers::load_from_vm(&ctx, &mem, port_ptr, port_len)?).unwrap())
            .unwrap(),
    );
    let payload = super::helpers::load_from_vm(&ctx, &mem, payload_ptr, payload_len).unwrap();

    let target_instance_id = super::helpers::load_instance_id_from_vm(&ctx, &mem, instance_node_id_ptr, instance_component_id_ptr)?;

    embassy_futures::block_on(caller.data().host.cast_raw(target_instance_id, target_port, payload)).map_err(|_| wasmi::Error::new("Cast Error"))
}

pub fn call_raw(
    mut caller: wasmi::Caller<'_, GuestAPI>,
    instance_node_id_ptr: i32,
    instance_component_id_ptr: i32,
    port_ptr: i32,
    port_len: i32,
    payload_ptr: i32,
    payload_len: i32,
    out_ptr_ptr: i32,
    out_len_ptr: i32,
) -> Result<i32, wasmi::Error> {
    let mem = get_memory(&mut caller)?;
    let alloc = get_alloc(&mut caller)?;
    let ctx = caller.as_context();

    let target_instance_id = super::helpers::load_instance_id_from_vm(&ctx, &mem, instance_node_id_ptr, instance_component_id_ptr)?;

    let target_port = edgeless_api_core::port::Port(
        heapless::String::from_utf8(
            heapless::Vec::<u8, 32>::from_slice(super::helpers::load_from_vm(&ctx, &mem, port_ptr, port_len).unwrap()).unwrap(),
        )
        .unwrap(),
    );
    let payload = super::helpers::load_from_vm(&ctx, &mem, payload_ptr, payload_len).unwrap();

    let call_ret = embassy_futures::block_on(caller.data().host.call_raw(target_instance_id, target_port, payload))
        .map_err(|_| wasmi::Error::new("Call Error"))?;

    match call_ret {
        crate::dataplane::CallRet::NoReply => Ok(0),
        crate::dataplane::CallRet::Reply(data) => {
            let len = data.len();

            let data_ptr = super::helpers::copy_to_vm(&mut caller.as_context_mut(), &mem, &alloc, &data)?;
            super::helpers::copy_to_vm_ptr(&mut caller.as_context_mut(), &mem, out_ptr_ptr, &data_ptr.to_le_bytes())?;
            super::helpers::copy_to_vm_ptr(&mut caller.as_context_mut(), &mem, out_len_ptr, &len.to_le_bytes())?;

            Ok(1)
        }
        crate::dataplane::CallRet::Err => Ok(2),
    }
}

pub fn cast(
    mut caller: wasmi::Caller<'_, GuestAPI>,
    target_ptr: i32,
    target_len: i32,
    payload_ptr: i32,
    payload_len: i32,
) -> Result<(), wasmi::Error> {
    let mem = get_memory(&mut caller)?;
    let ctx = caller.as_context();

    let target = super::helpers::load_str_from_vm(&ctx, &mem, target_ptr, target_len)?;
    let payload = super::helpers::load_from_vm(&ctx, &mem, payload_ptr, payload_len)?;

    embassy_futures::block_on(caller.data().host.cast_alias(target, &payload)).map_err(|_| wasmi::Error::new("Cast Error"))
}

pub fn call(
    mut caller: wasmi::Caller<'_, GuestAPI>,
    target_ptr: i32,
    target_len: i32,
    payload_ptr: i32,
    payload_len: i32,
    out_ptr_ptr: i32,
    out_len_ptr: i32,
) -> Result<i32, wasmi::Error> {
    let mem = get_memory(&mut caller)?;
    let alloc = get_alloc(&mut caller)?;
    let ctx = caller.as_context();

    let target = super::helpers::load_str_from_vm(&ctx, &mem, target_ptr, target_len)?;
    let payload = super::helpers::load_from_vm(&ctx, &mem, payload_ptr, payload_len)?;

    let call_ret = embassy_futures::block_on(caller.data().host.call_alias(target, payload)).map_err(|_| wasmi::Error::new("Call Error"))?;

    match call_ret {
        crate::dataplane::CallRet::NoReply => Ok(0),
        crate::dataplane::CallRet::Reply(data) => {
            let len = data.len();

            let data_ptr = super::helpers::copy_to_vm(&mut caller.as_context_mut(), &mem, &alloc, &data)?;
            super::helpers::copy_to_vm_ptr(&mut caller.as_context_mut(), &mem, out_ptr_ptr, &data_ptr.to_le_bytes())?;
            super::helpers::copy_to_vm_ptr(&mut caller.as_context_mut(), &mem, out_len_ptr, &len.to_le_bytes())?;

            Ok(1)
        }
        crate::dataplane::CallRet::Err => Ok(2),
    }
}

pub fn delayed_cast(
    mut caller: wasmi::Caller<'_, GuestAPI>,
    delay_ms: i64,
    target_ptr: i32,
    target_len: i32,
    payload_ptr: i32,
    payload_len: i32,
) -> Result<(), wasmi::Error> {
    let mem = get_memory(&mut caller)?;
    let ctx = caller.as_context();

    let target = super::helpers::load_str_from_vm(&ctx, &mem, target_ptr, target_len)?;
    let payload = super::helpers::load_from_vm(&ctx, &mem, payload_ptr, payload_len)?;

    embassy_futures::block_on(caller.data().host.delayed_cast(delay_ms as u64, target, payload)).map_err(|_| wasmi::Error::new("Delayed Cast Error"))
}

pub fn sync(mut caller: wasmi::Caller<'_, GuestAPI>, state_ptr: i32, state_len: i32) -> Result<(), wasmi::Error> {
    let mem = get_memory(&mut caller)?;
    let ctx = caller.as_context();

    let state = super::helpers::load_from_vm(&ctx, &mem, state_ptr, state_len).unwrap();

    embassy_futures::block_on(caller.data().host.sync(state)).map_err(|_| wasmi::Error::new("Sync Error"))
}

pub fn slf(mut caller: wasmi::Caller<'_, GuestAPI>, out_node_id_ptr: i32, out_component_id_ptr: i32) -> Result<(), wasmi::Error> {
    let mem = get_memory(&mut caller)?;

    let id = embassy_futures::block_on(caller.data().host.slf());

    super::helpers::copy_to_vm_ptr(&mut caller.as_context_mut(), &mem, out_node_id_ptr, id.node_id.as_bytes())?;
    super::helpers::copy_to_vm_ptr(&mut caller.as_context_mut(), &mem, out_component_id_ptr, id.function_id.as_bytes())?;

    Ok(())
}

pub(crate) fn get_memory(caller: &mut wasmi::Caller<'_, super::guest_api_binding::GuestAPI>) -> Result<wasmi::Memory, wasmi::Error> {
    caller
        .get_export("memory")
        .ok_or(wasmi::Error::new("memory error"))?
        .into_memory()
        .ok_or(wasmi::Error::new("memory error"))
}

pub(crate) fn get_alloc(caller: &mut wasmi::Caller<'_, super::guest_api_binding::GuestAPI>) -> Result<wasmi::TypedFunc<i32, i32>, wasmi::Error> {
    caller
        .get_export("edgeless_mem_alloc")
        .ok_or(wasmi::Error::new("alloc error"))?
        .into_func()
        .ok_or(wasmi::Error::new("alloc error"))?
        .typed::<i32, i32>(&caller)
        .map_err(|_| wasmi::Error::new("alloc error"))
}
