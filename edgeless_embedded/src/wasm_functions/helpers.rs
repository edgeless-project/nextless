// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

use core::borrow::BorrowMut;

pub(crate) fn copy_to_vm(
    ctx: &mut wasmi::StoreContextMut<'_, super::guest_api_binding::GuestAPI>,
    memory: &wasmi::Memory,
    alloc: &wasmi::TypedFunc<i32, i32>,
    data: &[u8],
) -> Result<i32, wasmi::Error> {
    let data_ptr = alloc
        .call(ctx.borrow_mut(), data.len() as i32)
        .map_err(|_| wasmi::Error::new("alloc error"))?;
    memory.data_mut(ctx.borrow_mut())[data_ptr as usize..(data_ptr as usize) + data.len()].copy_from_slice(data);
    Ok(data_ptr)
}

// This does not check the target length
pub(crate) fn copy_to_vm_ptr(
    ctx: &mut wasmi::StoreContextMut<'_, super::guest_api_binding::GuestAPI>,
    memory: &wasmi::Memory,
    target_ptr: i32,
    data: &[u8],
) -> Result<(), wasmi::Error> {
    memory.data_mut(ctx.borrow_mut())[target_ptr as usize..(target_ptr as usize) + data.len()].copy_from_slice(data);
    Ok(())
}

pub(crate) fn load_from_vm<'a>(
    ctx: &'a wasmi::StoreContext<'_, super::guest_api_binding::GuestAPI>,
    memory: &'a wasmi::Memory,
    data_ptr: i32,
    data_len: i32,
) -> Result<&'a [u8], wasmi::Error> {
    Ok(&memory.data(ctx)[data_ptr as usize..(data_ptr as usize) + data_len as usize])
}

pub(crate) fn load_str_from_vm<'a>(
    ctx: &'a wasmi::StoreContext<'_, super::guest_api_binding::GuestAPI>,
    memory: &'a wasmi::Memory,
    data_ptr: i32,
    data_len: i32,
) -> Result<&'a str, wasmi::Error> {
    core::str::from_utf8(load_from_vm(ctx, memory, data_ptr, data_len)?).map_err(|_| wasmi::Error::new("String Error"))
}

pub(crate) fn load_instance_id_from_vm(
    ctx: &wasmi::StoreContext<'_, super::guest_api_binding::GuestAPI>,
    memory: &wasmi::Memory,
    instance_node_id_ptr: i32,
    instance_component_id_ptr: i32,
) -> Result<edgeless_api_core::instance_id::InstanceId, wasmi::Error> {
    let node_id = load_from_vm(ctx, memory, instance_node_id_ptr, 16)?;
    let component_id = load_from_vm(ctx, memory, instance_component_id_ptr, 16)?;
    Ok(edgeless_api_core::instance_id::InstanceId {
        node_id: uuid::Uuid::from_bytes(node_id.try_into().map_err(|_| wasmi::Error::new("uuid error"))?),
        function_id: uuid::Uuid::from_bytes(component_id.try_into().map_err(|_| wasmi::Error::new("uuid error"))?),
    })
}

pub(crate) fn level_from_i32(lvl: i32) -> crate::wasm_functions::TelemetryLogLevel {
    match lvl {
        1 => crate::wasm_functions::TelemetryLogLevel::Error,
        2 => crate::wasm_functions::TelemetryLogLevel::Warn,
        3 => crate::wasm_functions::TelemetryLogLevel::Info,
        4 => crate::wasm_functions::TelemetryLogLevel::Debug,
        5 => crate::wasm_functions::TelemetryLogLevel::Trace,
        _ => {
            log::warn!("Function used unknown Log Level");
            crate::wasm_functions::TelemetryLogLevel::Error
        }
    }
}
