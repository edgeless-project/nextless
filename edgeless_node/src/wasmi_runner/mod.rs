// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

use wasmi::AsContextMut;

pub mod guest_api_binding;
mod helpers;
pub mod runtime;

#[cfg(test)]
pub mod test;
pub struct WASMIFunctionInstance {
    edgeless_mem_alloc: wasmi::TypedFunc<i32, i32>,
    edgeless_mem_free: wasmi::TypedFunc<(i32, i32), ()>,
    edgeless_mem_clear: wasmi::TypedFunc<(), ()>,
    #[allow(clippy::type_complexity)]
    edgefunctione_handle_call: wasmi::TypedFunc<
        (
            i32, // node_id_ptr
            i32, // component_id_ptr
            i32, // port_ptr,
            i32, // port_len
            i32, // payload_ptr
            i32, // payload_len
            i32, // out_ptr_ptr
            i32, // out_len_ptr
        ),
        i32, // Encoded CallRet
    >,
    edgefunctione_handle_cast: wasmi::TypedFunc<
        (
            i32, // node_id_ptr
            i32, // component_id_ptr
            i32, // port_ptr,
            i32, // port_len
            i32, // payload_ptr
            i32, // payload_len
        ),
        (),
    >,
    edgefunctione_handle_init: wasmi::TypedFunc<
        (
            i32, // payload_ptr
            i32, // payload_size
            i32, // serialized_state_ptr
            i32, // serialized_state_size
        ),
        (),
    >,
    edgefunctione_handle_stop: wasmi::TypedFunc<(), ()>,
    memory: wasmi::Memory,
    store: wasmi::Store<guest_api_binding::GuestAPI>,
}

#[async_trait::async_trait]
impl crate::base_runtime::FunctionInstance for WASMIFunctionInstance {
    async fn instantiate(
        _instance_id: &edgeless_api::function_instance::InstanceId,
        _runtime_configuration: std::collections::HashMap<String, String>,
        guest_api_host: &mut Option<crate::base_runtime::guest_api::GuestAPIHost>,
        code: &[u8],
    ) -> Result<Box<Self>, crate::base_runtime::FunctionInstanceError> {
        let _comfig = wasmi::Config::default();

        let engine = wasmi::Engine::default();
        let module = wasmi::Module::new(&engine, code)
            .map_err(|e| anyhow::anyhow!("Module creation error: {}", e))
            .map_err(crate::base_runtime::FunctionInstanceError::Internal)?;
        let mut store = wasmi::Store::new(
            &engine,
            guest_api_binding::GuestAPI {
                host: guest_api_host.take().expect("the impossible happened: no GuestAPIHost"),
            },
        );
        let mut linker = wasmi::Linker::<guest_api_binding::GuestAPI>::new(&engine);

        linker
            .define("env", "cast_raw_asm", wasmi::Func::wrap(&mut store, guest_api_binding::cast_raw))
            .map_err(|e| anyhow::anyhow!("Link cast_raw_asm error: {}", e))
            .map_err(crate::base_runtime::FunctionInstanceError::Internal)?;
        linker
            .define("env", "cast_asm", wasmi::Func::wrap(&mut store, guest_api_binding::cast))
            .map_err(|e| anyhow::anyhow!("Link cast_asm error: {}", e))
            .map_err(crate::base_runtime::FunctionInstanceError::Internal)?;
        linker
            .define("env", "call_raw_asm", wasmi::Func::wrap(&mut store, guest_api_binding::call_raw))
            .map_err(|e| anyhow::anyhow!("Link call_raw_asm error: {}", e))
            .map_err(crate::base_runtime::FunctionInstanceError::Internal)?;
        linker
            .define("env", "call_asm", wasmi::Func::wrap(&mut store, guest_api_binding::call))
            .map_err(|e| anyhow::anyhow!("Link call_asm error: {}", e))
            .map_err(crate::base_runtime::FunctionInstanceError::Internal)?;
        linker
            .define(
                "env",
                "telemetry_log_asm",
                wasmi::Func::wrap(&mut store, guest_api_binding::telemetry_log),
            )
            .map_err(|e| anyhow::anyhow!("Link telemetry_log_asm error: {}", e))
            .map_err(crate::base_runtime::FunctionInstanceError::Internal)?;
        linker
            .define("env", "slf_asm", wasmi::Func::wrap(&mut store, guest_api_binding::slf))
            .map_err(|e| anyhow::anyhow!("Link slf_asm error: {}", e))
            .map_err(crate::base_runtime::FunctionInstanceError::Internal)?;
        linker
            .define("env", "delayed_cast_asm", wasmi::Func::wrap(&mut store, guest_api_binding::delayed_cast))
            .map_err(|e| anyhow::anyhow!("Link delayed_cast_asm error: {}", e))
            .map_err(crate::base_runtime::FunctionInstanceError::Internal)?;
        linker
            .define("env", "sync_asm", wasmi::Func::wrap(&mut store, guest_api_binding::sync))
            .map_err(|e| anyhow::anyhow!("Link sync_asm error: {}", e))
            .map_err(crate::base_runtime::FunctionInstanceError::Internal)?;

        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| anyhow::anyhow!("Linker nstantiate error: {}", e))
            .map_err(crate::base_runtime::FunctionInstanceError::Internal)?
            .start(&mut store)
            .map_err(|e| anyhow::anyhow!("Linker start error: {}", e))
            .map_err(crate::base_runtime::FunctionInstanceError::Internal)?;

        Ok(Box::new(Self {
            edgeless_mem_alloc: instance
                .get_typed_func::<i32, i32>(&mut store, "edgeless_mem_alloc")
                .map_err(|e| anyhow::anyhow!("get function edgeless_mem_alloc error: {}", e))
                .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?,
            edgeless_mem_free: instance
                .get_typed_func::<(i32, i32), ()>(&mut store, "edgeless_mem_free")
                .map_err(|e| anyhow::anyhow!("get function edgeless_mem_free error: {}", e))
                .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?,
            edgeless_mem_clear: instance
                .get_typed_func::<(), ()>(&mut store, "edgeless_mem_clear")
                .map_err(|e| anyhow::anyhow!("get function edgeless_mem_clear error: {}", e))
                .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?,
            edgefunctione_handle_call: instance
                .get_typed_func::<(i32, i32, i32, i32, i32, i32, i32, i32), i32>(&mut store, "handle_call_asm")
                .map_err(|e| anyhow::anyhow!("get function handle_call_asm error: {}", e))
                .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?,
            edgefunctione_handle_cast: instance
                .get_typed_func::<(i32, i32, i32, i32, i32, i32), ()>(&mut store, "handle_cast_asm")
                .map_err(|e| anyhow::anyhow!("get function handle_call_asm error: {}", e))
                .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?,
            edgefunctione_handle_init: instance
                .get_typed_func::<(i32, i32, i32, i32), ()>(&mut store, "handle_init_asm")
                .map_err(|e| anyhow::anyhow!("get function handle_init_asm error: {}", e))
                .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?,
            edgefunctione_handle_stop: instance
                .get_typed_func::<(), ()>(&mut store, "handle_stop_asm")
                .map_err(|e| anyhow::anyhow!("get function handle_stop_asm error: {}", e))
                .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?,
            memory: instance
                .get_memory(&mut store, "memory")
                .ok_or(anyhow::anyhow!("Could not get memory"))
                .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?,
            store,
        }))
    }

    async fn init(&mut self, init_payload: Option<&str>, serialized_state: Option<&str>) -> Result<(), crate::base_runtime::FunctionInstanceError> {
        let (init_payload_ptr, init_payload_len) = match init_payload {
            Some(payload) => {
                let len = payload.len();
                let ptr = helpers::copy_to_vm(
                    &mut self.store.as_context_mut(),
                    &self.memory,
                    &self.edgeless_mem_alloc,
                    payload.as_bytes(),
                )
                .map_err(|e| anyhow::anyhow!("Copy payload to vm error: {}", e))
                .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;
                (ptr, len as i32)
            }
            None => (0i32, 0i32),
        };

        let (serialized_state_ptr, serialized_state_len) = match serialized_state {
            Some(state) => {
                let len = state.len();
                let ptr = helpers::copy_to_vm(&mut self.store.as_context_mut(), &self.memory, &self.edgeless_mem_alloc, state.as_bytes())
                    .map_err(|e| anyhow::anyhow!("Copy state to vm error: {}", e))
                    .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;
                (ptr, len as i32)
            }
            None => (0i32, 0i32),
        };

        let ret = tokio::task::block_in_place(|| {
            self.edgefunctione_handle_init
                .call(
                    &mut self.store,
                    (init_payload_ptr, init_payload_len, serialized_state_ptr, serialized_state_len),
                )
                .map_err(|e| anyhow::anyhow!("Call init failed: {}", e))
                .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;
            Ok(())
        });

        if init_payload_len > 0 {
            self.edgeless_mem_free
                .call(&mut self.store, (init_payload_ptr, init_payload_len))
                .map_err(|e| anyhow::anyhow!("Free payload error: {}", e))
                .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;
        }

        if serialized_state_len > 0 {
            self.edgeless_mem_free
                .call(&mut self.store, (serialized_state_ptr, serialized_state_len))
                .map_err(|e| anyhow::anyhow!("Free serialized state error: {}", e))
                .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;
        }

        ret
    }

    async fn cast(
        &mut self,
        src: &edgeless_api::function_instance::InstanceId,
        port: &str,
        msg: &[u8],
    ) -> Result<(), crate::base_runtime::FunctionInstanceError> {
        // Depending on the Function, we might employ a basic arena/bump allocator that we must reset at the end of a transaction.
        // This might be a noop if the function defines a working version of `edgeless_mem_free`.
        self.edgeless_mem_clear
            .call(&mut self.store, ())
            .map_err(|e| anyhow::anyhow!("Memory clear error: {}", e))
            .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;

        let component_id_ptr = helpers::copy_to_vm(
            &mut self.store.as_context_mut(),
            &self.memory,
            &self.edgeless_mem_alloc,
            src.function_id.as_bytes(),
        )
        .map_err(|e| anyhow::anyhow!("Copy component_id to vm error: {}", e))
        .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;

        let node_id_ptr = helpers::copy_to_vm(
            &mut self.store.as_context_mut(),
            &self.memory,
            &self.edgeless_mem_alloc,
            src.node_id.as_bytes(),
        )
        .map_err(|e| anyhow::anyhow!("Copy node_id to vm error: {}", e))
        .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;

        let port_len = port.len();
        let port_ptr = helpers::copy_to_vm(&mut self.store.as_context_mut(), &self.memory, &self.edgeless_mem_alloc, port.as_bytes())
            .map_err(|e| anyhow::anyhow!("Copy port to vm error: {}", e))
            .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;

        let payload_len = msg.len();
        let payload_ptr = helpers::copy_to_vm(&mut self.store.as_context_mut(), &self.memory, &self.edgeless_mem_alloc, msg)
            .map_err(|e| anyhow::anyhow!("Copy payload to vm error: {}", e))
            .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;

        let ret = tokio::task::block_in_place(|| {
            self.edgefunctione_handle_cast
                .call(
                    &mut self.store,
                    (node_id_ptr, component_id_ptr, port_ptr, port_len as i32, payload_ptr, payload_len as i32),
                )
                .map_err(|e| anyhow::anyhow!("Call cast error: {}", e))
                .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;
            Ok(())
        });

        self.edgeless_mem_free
            .call(&mut self.store, (component_id_ptr, 16))
            .map_err(|e| anyhow::anyhow!("Free component_id error: {}", e))
            .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;
        self.edgeless_mem_free
            .call(&mut self.store, (node_id_ptr, 16))
            .map_err(|e| anyhow::anyhow!("Free node_id error: {}", e))
            .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;
        if payload_len > 0 {
            self.edgeless_mem_free
                .call(&mut self.store, (payload_ptr, payload_len as i32))
                .map_err(|e| anyhow::anyhow!("Free payload error: {}", e))
                .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;
        }
        ret
    }

    async fn call(
        &mut self,
        src: &edgeless_api::function_instance::InstanceId,
        port: &str,
        msg: &[u8],
    ) -> Result<edgeless_dataplane::core::CallRet, crate::base_runtime::FunctionInstanceError> {
        self.edgeless_mem_clear
            .call(&mut self.store, ())
            .map_err(|e| anyhow::anyhow!("Memory clear error: {}", e))
            .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;

        let component_id_ptr = helpers::copy_to_vm(
            &mut self.store.as_context_mut(),
            &self.memory,
            &self.edgeless_mem_alloc,
            src.function_id.as_bytes(),
        )
        .map_err(|e| anyhow::anyhow!("Copy component_id to vm error: {}", e))
        .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;

        let node_id_ptr = helpers::copy_to_vm(
            &mut self.store.as_context_mut(),
            &self.memory,
            &self.edgeless_mem_alloc,
            src.node_id.as_bytes(),
        )
        .map_err(|e| anyhow::anyhow!("Copy node_id to vm error: {}", e))
        .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;

        let port_len = port.len();
        let port_ptr = helpers::copy_to_vm(&mut self.store.as_context_mut(), &self.memory, &self.edgeless_mem_alloc, port.as_bytes())
            .map_err(|e| anyhow::anyhow!("Copy port to vm error: {}", e))
            .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;

        let payload_len = msg.len();
        let payload_ptr = helpers::copy_to_vm(&mut self.store.as_context_mut(), &self.memory, &self.edgeless_mem_alloc, msg)
            .map_err(|e| anyhow::anyhow!("Copy payload to vm error: {}", e))
            .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;

        let out_ptr_ptr = self
            .edgeless_mem_alloc
            .call(&mut self.store, 4)
            .map_err(|e| anyhow::anyhow!("Output buffer ptr alloc error: {}", e))
            .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;

        let out_len_ptr = self
            .edgeless_mem_alloc
            .call(&mut self.store, 4)
            .map_err(|e| anyhow::anyhow!("Output buffer len alloc error: {}", e))
            .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;

        let callret_type = tokio::task::block_in_place(|| {
            self.edgefunctione_handle_call
                .call(
                    &mut self.store,
                    (
                        node_id_ptr,
                        component_id_ptr,
                        port_ptr,
                        port_len as i32,
                        payload_ptr,
                        payload_len as i32,
                        out_ptr_ptr,
                        out_len_ptr,
                    ),
                )
                .map_err(|e| anyhow::anyhow!("Call call error: {}", e))
                .map_err(crate::base_runtime::FunctionInstanceError::BadCode)
        })?;

        let ret = match callret_type {
            0 => Ok(edgeless_dataplane::core::CallRet::NoReply),
            1 => {
                // load the output pointer (inside the WASM memory) (layer of indirection to work around only using one return param)
                let out_ptr: [u8; 4] = self.memory.data_mut(&mut self.store)[out_ptr_ptr as usize..(out_ptr_ptr as usize) + 4]
                    .try_into()
                    .map_err(|e| anyhow::anyhow!("Output buffer ptr conversion error: {}", e))
                    .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;
                let out_ptr = i32::from_le_bytes(out_ptr);

                // load the output lenght (layer of indirection to work around only using one return param)
                let out_len: [u8; 4] = self.memory.data_mut(&mut self.store)[out_len_ptr as usize..(out_len_ptr as usize) + 4]
                    .try_into()
                    .map_err(|e| anyhow::anyhow!("Output buffer len conversion error: {}", e))
                    .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;
                let out_len = i32::from_le_bytes(out_len);

                // load the atual output param
                let out_raw = self.memory.data_mut(&mut self.store)[out_ptr as usize..(out_ptr as usize) + out_len as usize].to_vec();

                Ok(edgeless_dataplane::core::CallRet::Reply(out_raw.to_vec()))
            }
            _ => Ok(edgeless_dataplane::core::CallRet::Err),
        };

        self.edgeless_mem_free
            .call(&mut self.store, (component_id_ptr, 16))
            .map_err(|e| anyhow::anyhow!("Free component_id error: {}", e))
            .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;
        self.edgeless_mem_free
            .call(&mut self.store, (node_id_ptr, 16))
            .map_err(|e| anyhow::anyhow!("Free node_id error: {}", e))
            .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;
        // We don't need to free the data referred to by this pointer as we assume them to be stack-allocated.
        self.edgeless_mem_free
            .call(&mut self.store, (out_ptr_ptr, 4))
            .map_err(|e| anyhow::anyhow!("Free output buffer ptr error: {}", e))
            .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;
        // We don't need to free the data referred to by this pointer as we assume them to be stack-allocated.
        self.edgeless_mem_free
            .call(&mut self.store, (out_len_ptr, 4))
            .map_err(|e| anyhow::anyhow!("Free output buffer len error: {}", e))
            .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;

        if payload_len > 0 {
            self.edgeless_mem_free
                .call(&mut self.store, (payload_ptr, payload_len as i32))
                .map_err(|e| anyhow::anyhow!("Free payload error: {}", e))
                .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;
        }

        ret
    }

    async fn stop(&mut self) -> Result<(), crate::base_runtime::FunctionInstanceError> {
        self.edgeless_mem_clear
            .call(&mut self.store, ())
            .map_err(|e| anyhow::anyhow!("Memory clear Error: {}", e))
            .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;
        tokio::task::block_in_place(|| {
            self.edgefunctione_handle_stop
                .call(&mut self.store, ())
                .map_err(|e| anyhow::anyhow!("Call stop error: {}", e))
                .map_err(crate::base_runtime::FunctionInstanceError::BadCode)
        })?;
        Ok(())
    }
}
