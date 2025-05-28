// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
use wasmtime::AsContextMut;

/// FunctionInstance implementation allowing to execute functions defined as WASM components.
/// Note that this only contains the WASM specific bindings, while the base_runtime provides the generic runtime functionality.
pub struct WASMFunctionInstance {
    edgeless_mem_alloc: wasmtime::TypedFunc<i32, i32>,
    edgeless_mem_free: wasmtime::TypedFunc<(i32, i32), ()>,
    edgeless_mem_clear: wasmtime::TypedFunc<(), ()>,
    #[allow(clippy::type_complexity)]
    edgefunctione_handle_call: wasmtime::TypedFunc<
        (
            i32, // node_id_ptr
            i32, // component_id_ptr
            i32, // port_ptr
            i32, // port_len
            i32, // payload_ptr
            i32, // payload_len
            i32, // out_ptr_ptr
            i32, // out_len_ptr
        ),
        i32, // Encoded CallRet
    >,
    edgefunctione_handle_cast: wasmtime::TypedFunc<
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
    edgefunctione_handle_init: wasmtime::TypedFunc<
        (
            i32, // payload_ptr
            i32, // payload_size
            i32, // serialized_state_ptr
            i32, // serialized_state_size
        ),
        (),
    >,
    edgefunctione_handle_stop: wasmtime::TypedFunc<(), ()>,
    memory: wasmtime::Memory,
    store: wasmtime::Store<super::guest_api_binding::GuestAPI>,
}

#[async_trait::async_trait]
impl crate::base_runtime::FunctionInstance for WASMFunctionInstance {
    async fn instantiate(
        _instance_id: &edgeless_api::function_instance::InstanceId,
        _runtime_configuration: std::collections::HashMap<String, String>,
        guest_api_host: crate::base_runtime::guest_api::GuestAPIHost,
        code: &[u8],
    ) -> Result<Box<Self>, crate::base_runtime::FunctionInstanceError> {
        let mut config = wasmtime::Config::new();
        config.async_support(true);
        config.wasm_bulk_memory(true);
        config.wasm_function_references(true);
        let engine = wasmtime::Engine::new(&config).map_err(crate::base_runtime::FunctionInstanceError::Internal)?;
        let module = wasmtime::Module::from_binary(&engine, code).map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;
        let mut linker = wasmtime::Linker::new(&engine);

        let mut store: wasmtime::Store<super::guest_api_binding::GuestAPI> = wasmtime::Store::new(
            &engine,
            super::guest_api_binding::GuestAPI {
                host: guest_api_host,
                wgpu_wrapper: super::gpu_binding::GPUWrapper::new(),
            },
        );

        linker
            .func_wrap_async(
                "env",
                "cast_raw_asm",
                |store, (instance_node_id_ptr, instance_component_id_ptr, port_ptr, port_len, payload_ptr, payload_len)| {
                    Box::new(super::guest_api_binding::cast_raw(
                        store,
                        instance_node_id_ptr,
                        instance_component_id_ptr,
                        port_ptr,
                        port_len,
                        payload_ptr,
                        payload_len,
                    ))
                },
            )
            .map_err(crate::base_runtime::FunctionInstanceError::Internal)?;
        linker
            .func_wrap_async("env", "cast_asm", |store, (target_ptr, target_len, payload_ptr, payload_len)| {
                Box::new(super::guest_api_binding::cast(store, target_ptr, target_len, payload_ptr, payload_len))
            })
            .map_err(crate::base_runtime::FunctionInstanceError::Internal)?;
        linker
            .func_wrap_async(
                "env",
                "call_raw_asm",
                |store, (instance_node_id_ptr, instance_component_id_ptr, port_ptr, port_len, payload_ptr, payload_len, out_ptr_ptr, out_len_ptr)| {
                    Box::new(super::guest_api_binding::call_raw(
                        store,
                        instance_node_id_ptr,
                        instance_component_id_ptr,
                        port_ptr,
                        port_len,
                        payload_ptr,
                        payload_len,
                        out_ptr_ptr,
                        out_len_ptr,
                    ))
                },
            )
            .map_err(crate::base_runtime::FunctionInstanceError::Internal)?;
        linker
            .func_wrap_async(
                "env",
                "call_asm",
                |store, (target_ptr, target_len, payload_ptr, payload_len, out_ptr_ptr, out_len_ptr)| {
                    Box::new(super::guest_api_binding::call(
                        store,
                        target_ptr,
                        target_len,
                        payload_ptr,
                        payload_len,
                        out_ptr_ptr,
                        out_len_ptr,
                    ))
                },
            )
            .map_err(crate::base_runtime::FunctionInstanceError::Internal)?;
        linker
            .func_wrap_async("env", "telemetry_log_asm", |store, (level, target_ptr, target_len, msg_ptr, msg_len)| {
                Box::new(super::guest_api_binding::telemetry_log(
                    store, level, target_ptr, target_len, msg_ptr, msg_len,
                ))
            })
            .map_err(crate::base_runtime::FunctionInstanceError::Internal)?;
        linker
            .func_wrap_async("env", "slf_asm", |store, (out_node_id_ptr, out_component_id_ptr)| {
                Box::new(super::guest_api_binding::slf(store, out_node_id_ptr, out_component_id_ptr))
            })
            .map_err(crate::base_runtime::FunctionInstanceError::Internal)?;
        linker
            .func_wrap_async(
                "env",
                "delayed_cast_asm",
                |store, (delay_ms, target_ptr, target_len, payload_ptr, payload_len)| {
                    Box::new(super::guest_api_binding::delayed_cast(
                        store,
                        delay_ms,
                        target_ptr,
                        target_len,
                        payload_ptr,
                        payload_len,
                    ))
                },
            )
            .map_err(crate::base_runtime::FunctionInstanceError::Internal)?;
        linker
            .func_wrap_async("env", "sync_asm", |store, (state_ptr, state_len)| {
                Box::new(super::guest_api_binding::sync(store, state_ptr, state_len))
            })
            .map_err(crate::base_runtime::FunctionInstanceError::Internal)?;
        linker
            .func_wrap_async("env", "webgpu_instance_new", |store, ()| {
                Box::new(super::guest_api_binding::wgpu_instance_new(store))
            })
            .map_err(crate::base_runtime::FunctionInstanceError::Internal)?;
        linker
            .func_wrap_async("env", "webgpu_instance_poll_all", |store, (instance_id, forced_wait)| {
                Box::new(super::guest_api_binding::wgpu_instance_poll_all(store, instance_id, forced_wait))
            })
            .map_err(crate::base_runtime::FunctionInstanceError::Internal)?;
        linker
            .func_wrap_async("env", "webgpu_instance_adapter_create", |store, (instance_id,)| {
                Box::new(super::guest_api_binding::webgpu_instance_adapter_create(store, instance_id))
            })
            .map_err(crate::base_runtime::FunctionInstanceError::Internal)?;
        linker
            .func_wrap_async("env", "webgpu_instance_wgsl_language_features", |store, (instance_id,)| {
                Box::new(super::guest_api_binding::webgpu_instance_wgsl_language_features(store, instance_id))
            })
            .map_err(crate::base_runtime::FunctionInstanceError::Internal)?;
        linker
            .func_wrap_async(
                "env",
                "webgpu_adapter_device_create",
                |store, (instance_id, out_device_id, out_queue_id)| {
                    log::info!("D1");
                    Box::new(super::guest_api_binding::webgpu_adapter_device_create(
                        store,
                        instance_id,
                        out_device_id,
                        out_queue_id,
                    ))
                },
            )
            .map_err(crate::base_runtime::FunctionInstanceError::Internal)?;
        linker
            .func_wrap(
                "env",
                "webgpu_device_create_shader_module",
                super::guest_api_binding::webgpu_device_create_shader_module,
            )
            .map_err(crate::base_runtime::FunctionInstanceError::Internal)?;
        linker
            .func_wrap(
                "env",
                "webgpu_device_create_bind_group_layout",
                super::guest_api_binding::webgpu_device_create_bind_group_layout,
            )
            .map_err(crate::base_runtime::FunctionInstanceError::Internal)?;
        linker
            .func_wrap(
                "env",
                "webgpu_device_create_bind_group",
                super::guest_api_binding::webgpu_device_create_bind_group,
            )
            .map_err(crate::base_runtime::FunctionInstanceError::Internal)?;
        linker
            .func_wrap(
                "env",
                "webgpu_device_create_dipatch_pipeline_layout",
                super::guest_api_binding::webgpu_device_create_dipatch_pipeline_layout,
            )
            .map_err(crate::base_runtime::FunctionInstanceError::Internal)?;
        linker
            .func_wrap(
                "env",
                "webgpu_device_create_compute_pipeline",
                super::guest_api_binding::webgpu_device_create_compute_pipeline,
            )
            .map_err(crate::base_runtime::FunctionInstanceError::Internal)?;
        linker
            .func_wrap(
                "env",
                "webgpu_device_create_buffer",
                super::guest_api_binding::webgpu_device_create_buffer,
            )
            .map_err(crate::base_runtime::FunctionInstanceError::Internal)?;
        linker
            .func_wrap(
                "env",
                "webgpu_device_create_command_encoder",
                super::guest_api_binding::webgpu_device_create_command_encoder,
            )
            .map_err(crate::base_runtime::FunctionInstanceError::Internal)?;
        linker
            .func_wrap("env", "webgpu_device_poll", super::guest_api_binding::webgpu_device_poll)
            .map_err(crate::base_runtime::FunctionInstanceError::Internal)?;
        linker
            .func_wrap("env", "webgpu_queue_write_buffer", super::guest_api_binding::webgpu_queue_write_buffer)
            .map_err(crate::base_runtime::FunctionInstanceError::Internal)?;
        linker
            .func_wrap("env", "webgpu_queue_submit", super::guest_api_binding::webgpu_queue_submit)
            .map_err(crate::base_runtime::FunctionInstanceError::Internal)?;
        linker
            .func_wrap(
                "env",
                "webgpu_queue_get_timestamp_period",
                super::guest_api_binding::webgpu_queue_get_timestamp_period,
            )
            .map_err(crate::base_runtime::FunctionInstanceError::Internal)?;
        linker
            .func_wrap("env", "webgpu_buffer_map_async", super::guest_api_binding::webgpu_buffer_map_async)
            .map_err(crate::base_runtime::FunctionInstanceError::Internal)?;
        linker
            .func_wrap(
                "env",
                "webgpu_ce_copy_buffer_to_buffer",
                super::guest_api_binding::webgpu_ce_copy_buffer_to_buffer,
            )
            .map_err(crate::base_runtime::FunctionInstanceError::Internal)?;
        linker
            .func_wrap(
                "env",
                "webgpu_ce_begin_compute_pass",
                super::guest_api_binding::webgpu_ce_begin_compute_pass,
            )
            .map_err(crate::base_runtime::FunctionInstanceError::Internal)?;
        linker
            .func_wrap("env", "webgpu_ce_finish", super::guest_api_binding::webgpu_ce_finish)
            .map_err(crate::base_runtime::FunctionInstanceError::Internal)?;
        linker
            .func_wrap("env", "webgpu_cp_set_pipeline", super::guest_api_binding::webgpu_cp_set_pipeline)
            .map_err(crate::base_runtime::FunctionInstanceError::Internal)?;
        linker
            .func_wrap("env", "webgpu_cp_set_bind_group", super::guest_api_binding::webgpu_cp_set_bind_group)
            .map_err(crate::base_runtime::FunctionInstanceError::Internal)?;
        linker
            .func_wrap(
                "env",
                "webgpu_cp_dispatch_workgroups",
                super::guest_api_binding::webgpu_cp_dispatch_workgroups,
            )
            .map_err(crate::base_runtime::FunctionInstanceError::Internal)?;
        linker
            .func_wrap(
                "env",
                "webgpu_cp_dispatch_workgroups_indirect",
                super::guest_api_binding::webgpu_cp_dispatch_workgroups_indirect,
            )
            .map_err(crate::base_runtime::FunctionInstanceError::Internal)?;
        linker
            .func_wrap(
                "env",
                "webgpu_cp_set_push_constants",
                super::guest_api_binding::webgpu_cp_set_push_constants,
            )
            .map_err(crate::base_runtime::FunctionInstanceError::Internal)?;
        // linker
        //     .func_wrap(
        //         "env",
        //         "webgpu_buffer_get_mapped_range",
        //         super::guest_api_binding::webgpu_buffer_get_mapped_range,
        //     )
        //     .map_err(|e| crate::base_runtime::FunctionInstanceError::InternalError(e.to_string()))?;;
        linker
            .func_wrap(
                "env",
                "webgpu_buffer_mapped_range_read",
                super::guest_api_binding::webgpu_buffer_mapped_range_read,
            )
            .map_err(crate::base_runtime::FunctionInstanceError::Internal)?;
        linker
            .func_wrap(
                "env",
                "webgpu_buffer_mapped_range_write",
                super::guest_api_binding::webgpu_buffer_mapped_range_write,
            )
            .map_err(crate::base_runtime::FunctionInstanceError::Internal)?;
        linker
            .func_wrap("env", "webgpu_cp_drop", super::guest_api_binding::webgpu_cp_drop)
            .map_err(crate::base_runtime::FunctionInstanceError::Internal)?;
        linker
            .func_wrap("env", "webgpu_buffer_unmap", super::guest_api_binding::webgpu_buffer_unmap)
            .map_err(crate::base_runtime::FunctionInstanceError::Internal)?;
        linker
            .func_wrap(
                "env",
                "webgpu_compute_pipeline_get_bind_group_layout",
                super::guest_api_binding::webgpu_compute_pipeline_get_bind_group_layout,
            )
            .map_err(crate::base_runtime::FunctionInstanceError::Internal)?;
        linker
            .func_wrap("env", "webgpu_drop", super::guest_api_binding::webgpu_drop)
            .map_err(crate::base_runtime::FunctionInstanceError::Internal)?;
        let instance = linker.instantiate_async(&mut store, &module).await.unwrap();

        Ok(Box::new(Self {
            edgeless_mem_alloc: instance
                .get_typed_func::<i32, i32>(&mut store, "edgeless_mem_alloc")
                .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?,
            edgeless_mem_free: instance
                .get_typed_func::<(i32, i32), ()>(&mut store, "edgeless_mem_free")
                .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?,
            edgeless_mem_clear: instance
                .get_typed_func::<(), ()>(&mut store, "edgeless_mem_clear")
                .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?,
            edgefunctione_handle_call: instance
                .get_typed_func::<(i32, i32, i32, i32, i32, i32, i32, i32), i32>(&mut store, "handle_call_asm")
                .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?,
            edgefunctione_handle_cast: instance
                .get_typed_func::<(i32, i32, i32, i32, i32, i32), ()>(&mut store, "handle_cast_asm")
                .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?,
            edgefunctione_handle_init: instance
                .get_typed_func::<(i32, i32, i32, i32), ()>(&mut store, "handle_init_asm")
                .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?,
            edgefunctione_handle_stop: instance
                .get_typed_func::<(), ()>(&mut store, "handle_stop_asm")
                .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?,
            memory: instance
                .get_memory(&mut store, "memory")
                .ok_or(anyhow::anyhow!("could not get memory"))
                .map_err(crate::base_runtime::FunctionInstanceError::Internal)?,
            store,
        }))
    }

    async fn init(&mut self, init_payload: Option<&str>, serialized_state: Option<&[u8]>) -> Result<(), crate::base_runtime::FunctionInstanceError> {
        let (init_payload_ptr, init_payload_len) = match init_payload {
            Some(payload) => {
                let len = payload.len();
                let ptr = super::helpers::copy_to_vm(
                    &mut self.store.as_context_mut(),
                    &self.memory,
                    &self.edgeless_mem_alloc,
                    payload.as_bytes(),
                )
                .await
                .map_err(|e| e.context("Could not copy payload to vm"))
                .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;
                (ptr, len as i32)
            }
            None => (0i32, 0i32),
        };

        let (serialized_state_ptr, serialized_state_len) = match serialized_state {
            Some(state) => {
                let len = state.len();
                let ptr = super::helpers::copy_to_vm(&mut self.store.as_context_mut(), &self.memory, &self.edgeless_mem_alloc, state)
                    .await
                    .map_err(|e| e.context("Could not copy serialized_state to vm"))
                    .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;
                (ptr, len as i32)
            }
            None => (0i32, 0i32),
        };

        let ret = {
            self.edgefunctione_handle_init
                .call_async(
                    &mut self.store,
                    (init_payload_ptr, init_payload_len, serialized_state_ptr, serialized_state_len),
                )
                .await
                .map_err(|e| e.context("Init call failed"))
                .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;
            Ok(())
        };

        if init_payload_len > 0 {
            self.edgeless_mem_free
                .call_async(&mut self.store, (init_payload_ptr, init_payload_len))
                .await
                .map_err(|e| e.context("Could not free payload memory"))
                .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;
        }

        if serialized_state_len > 0 {
            self.edgeless_mem_free
                .call_async(&mut self.store, (serialized_state_ptr, serialized_state_len))
                .await
                .map_err(|e| e.context("Could not free serialized_state memory"))
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
            .call_async(&mut self.store, ())
            .await
            .map_err(|e| e.context("Could not clear memory"))
            .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;

        let component_id_ptr = super::helpers::copy_to_vm(
            &mut self.store.as_context_mut(),
            &self.memory,
            &self.edgeless_mem_alloc,
            src.function_id.as_bytes(),
        )
        .await
        .map_err(|e| e.context("Could copy component_id to vm"))
        .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;

        let node_id_ptr = super::helpers::copy_to_vm(
            &mut self.store.as_context_mut(),
            &self.memory,
            &self.edgeless_mem_alloc,
            src.node_id.as_bytes(),
        )
        .await
        .map_err(|e| e.context("Could copy node_id to vm"))
        .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;

        let payload_len = msg.len();
        let payload_ptr = super::helpers::copy_to_vm(&mut self.store.as_context_mut(), &self.memory, &self.edgeless_mem_alloc, msg)
            .await
            .map_err(|e| e.context("Could copy payload to vm"))
            .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;

        let port_len = port.len();
        let port_ptr = super::helpers::copy_to_vm(&mut self.store.as_context_mut(), &self.memory, &self.edgeless_mem_alloc, port.as_bytes())
            .await
            .map_err(|e| e.context("Could copy port to vm"))
            .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;

        let ret = {
            self.edgefunctione_handle_cast
                .call_async(
                    &mut self.store,
                    (node_id_ptr, component_id_ptr, port_ptr, port_len as i32, payload_ptr, payload_len as i32),
                )
                .await
                .map_err(|e| e.context("Cast call failed"))
                .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;
            Ok(())
        };

        self.edgeless_mem_free
            .call_async(&mut self.store, (component_id_ptr, 16))
            .await
            .map_err(|e| e.context("Could not free component_id memory"))
            .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;
        self.edgeless_mem_free
            .call_async(&mut self.store, (node_id_ptr, 16))
            .await
            .map_err(|e| e.context("Could not free node_id memory"))
            .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;
        if payload_len > 0 {
            self.edgeless_mem_free
                .call_async(&mut self.store, (payload_ptr, payload_len as i32))
                .await
                .map_err(|e| e.context("Could not free payload memory"))
                .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;
        }
        if port_len > 0 {
            self.edgeless_mem_free
                .call_async(&mut self.store, (port_ptr, port_len as i32))
                .await
                .map_err(|e| e.context("Could not free port memory"))
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
            .call_async(&mut self.store, ())
            .await
            .map_err(|e| e.context("Could not clear memory"))
            .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;

        let component_id_ptr = super::helpers::copy_to_vm(
            &mut self.store.as_context_mut(),
            &self.memory,
            &self.edgeless_mem_alloc,
            src.function_id.as_bytes(),
        )
        .await
        .map_err(|e| e.context("Could not copy compoent_id to vm"))
        .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;

        let node_id_ptr = super::helpers::copy_to_vm(
            &mut self.store.as_context_mut(),
            &self.memory,
            &self.edgeless_mem_alloc,
            src.node_id.as_bytes(),
        )
        .await
        .map_err(|e| e.context("Could not copy node_id to vm"))
        .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;

        let payload_len = msg.len();
        let payload_ptr = super::helpers::copy_to_vm(&mut self.store.as_context_mut(), &self.memory, &self.edgeless_mem_alloc, msg)
            .await
            .map_err(|e| e.context("Could not copy payload to vm"))
            .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;

        let port_len = port.len();
        let port_ptr = super::helpers::copy_to_vm(&mut self.store.as_context_mut(), &self.memory, &self.edgeless_mem_alloc, port.as_bytes())
            .await
            .map_err(|e| e.context("Could not copy port to vm"))
            .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;

        let out_ptr_ptr = self
            .edgeless_mem_alloc
            .call_async(&mut self.store, 4)
            .await
            .map_err(|e| e.context("Could not allocate memory for output buffer"))
            .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;

        let out_len_ptr = self
            .edgeless_mem_alloc
            .call_async(&mut self.store, 4)
            .await
            .map_err(|e| e.context("Could not allocate memory for output len"))
            .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;

        let callret_type = self
            .edgefunctione_handle_call
            .call_async(
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
            .await
            .map_err(|e| e.context("Call call failed"))
            .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;

        let ret = match callret_type {
            0 => Ok(edgeless_dataplane::core::CallRet::NoReply),
            1 => {
                // load the output pointer (inside the WASM memory) (layer of indirection to work around only using one return param)
                let out_ptr: [u8; 4] = self.memory.data_mut(&mut self.store)[out_ptr_ptr as usize..(out_ptr_ptr as usize) + 4]
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("Could not convert output buffer pointer"))
                    .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;
                let out_ptr = i32::from_le_bytes(out_ptr);

                // load the output lenght (layer of indirection to work around only using one return param)
                let out_len: [u8; 4] = self.memory.data_mut(&mut self.store)[out_len_ptr as usize..(out_len_ptr as usize) + 4]
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("Could not convert output len pointer"))
                    .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;
                let out_len = i32::from_le_bytes(out_len);

                // load the atual output param
                let out_raw = self.memory.data_mut(&mut self.store)[out_ptr as usize..(out_ptr as usize) + out_len as usize].to_vec();
                let out = out_raw;
                Ok(edgeless_dataplane::core::CallRet::Reply(out))
            }
            _ => Ok(edgeless_dataplane::core::CallRet::Err),
        };

        self.edgeless_mem_free
            .call_async(&mut self.store, (component_id_ptr, 16))
            .await
            .map_err(|e| e.context("Could not free component_id memory"))
            .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;
        self.edgeless_mem_free
            .call_async(&mut self.store, (node_id_ptr, 16))
            .await
            .map_err(|e| e.context("Could not free node_id memory"))
            .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;
        // We don't need to free the data referred to by this pointer as we assume them to be stack-allocated.
        self.edgeless_mem_free
            .call_async(&mut self.store, (out_ptr_ptr, 4))
            .await
            .map_err(|e| e.context("Could not free output buffer ptr memory"))
            .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;
        // We don't need to free the data referred to by this pointer as we assume them to be stack-allocated.
        self.edgeless_mem_free
            .call_async(&mut self.store, (out_len_ptr, 4))
            .await
            .map_err(|e| e.context("Could not free output buffer len memory"))
            .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;

        if payload_len > 0 {
            self.edgeless_mem_free
                .call_async(&mut self.store, (payload_ptr, payload_len as i32))
                .await
                .map_err(|e| e.context("Could not free payload memory"))
                .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;
        }

        if port_len > 0 {
            self.edgeless_mem_free
                .call_async(&mut self.store, (port_ptr, port_len as i32))
                .await
                .map_err(|e| e.context("Could not free port memory"))
                .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;
        }

        ret
    }

    async fn stop(&mut self) -> Result<(), crate::base_runtime::FunctionInstanceError> {
        self.edgeless_mem_clear
            .call_async(&mut self.store, ())
            .await
            .map_err(|e| e.context("Could not clear memory"))
            .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;
        self.edgefunctione_handle_stop
            .call_async(&mut self.store, ())
            .await
            .map_err(|e| e.context("Stop call failed"))
            .map_err(crate::base_runtime::FunctionInstanceError::BadCode)?;
        Ok(())
    }
}
