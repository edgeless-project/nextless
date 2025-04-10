// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

use crate::code_store::ImageEntry;
use core::str::FromStr;
use wasmi::AsContextMut;

pub mod guest_api;
pub mod guest_api_binding;
pub mod helpers;

#[derive(Clone, Debug)]
pub enum FunctionInstanceError {
    BadCode,
    InternalError,
}

#[derive(Debug, PartialEq, Eq)]
pub enum TelemetryLogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

pub struct WasmiRuntime {
    functions: alloc::vec::Vec<WasmiFunctionInstanceWrapper>,
    agent: Option<crate::agent::EmbeddedAgent>,
}

struct WasmiFunctionInstanceWrapper {
    instance_id: edgeless_api_core::instance_id::InstanceId,
    inner: WasmiFunctionInstance,
}

impl Default for WasmiRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl WasmiRuntime {
    pub fn new() -> Self {
        Self {
            functions: alloc::vec::Vec::new(),
            agent: None,
        }
    }

    pub async fn has_instance(&self, id: &edgeless_api_core::instance_id::InstanceId) -> bool {
        for i in &self.functions {
            if &i.instance_id == id {
                return true;
            }
        }

        false
    }

    pub async fn launch(&mut self, agent: crate::agent::EmbeddedAgent) {
        self.agent = Some(agent);
    }
}

impl crate::invocation::InvocationAPI for WasmiRuntime {
    async fn handle(&mut self, event: edgeless_api_core::invocation::Event) -> Result<edgeless_api_core::invocation::LinkProcessingResult, ()> {
        if let Some(f) = self.functions.iter_mut().find(|f| f.instance_id == event.target) {
            return f.handle(event).await;
        }
        Ok(edgeless_api_core::invocation::LinkProcessingResult::PASSED)
    }
}

impl crate::function_instance::FunctionInstanceAPI for WasmiRuntime {
    async fn start_function(
        &mut self,
        instance_specification: edgeless_api_core::function_instance::EncodedFunctionInstanceSpecification<'_>,
    ) -> Result<(), edgeless_api_core::common::ErrorResponse> {
        let image = self
            .agent
            .as_mut()
            .unwrap()
            .code_store()
            .get_image(&instance_specification.class)
            .await
            .expect("Bad Image");
        let image2 = image.image().unwrap();

        if self.functions.iter().any(|f| f.instance_id == instance_specification.instance_id) {
            return Err(edgeless_api_core::common::ErrorResponse {
                summary: "Function Id Exists",
                detail: None,
            });
        }

        let output_mapping = instance_specification
            .output_mapping
            .into_iter()
            .map(|(k, v)| (edgeless_api_core::port::Port(heapless::String::<32>::from_str(k).unwrap()), v))
            .collect();

        let mut inner_fun = WasmiFunctionInstance::instantiate(
            guest_api::GuestAPIHost {
                instance_id: instance_specification.instance_id,
                data_plane: crate::dataplane::EmbeddedDataplaneHandle::new(
                    instance_specification.instance_id,
                    self.agent.clone().unwrap(),
                    output_mapping,
                ),
            },
            image2.read(),
        )
        .await
        .unwrap();

        inner_fun.init(None, None).await.unwrap();

        let fun = WasmiFunctionInstanceWrapper {
            inner: inner_fun,
            instance_id: instance_specification.instance_id,
        };

        self.functions.push(fun);

        Ok(())
    }

    async fn stop_function(
        &mut self,
        function_id: edgeless_api_core::instance_id::InstanceId,
    ) -> Result<(), edgeless_api_core::common::ErrorResponse> {
        if let Some(f) = self.functions.iter_mut().find(|f| f.instance_id == function_id) {
            f.inner.stop().await.unwrap();
        }
        self.functions.retain(|f| f.instance_id != function_id);
        Ok(())
    }

    async fn patch_function(
        &mut self,
        patch_reg: edgeless_api_core::resource_configuration::EncodedPatchRequest<'_>,
    ) -> Result<(), edgeless_api_core::common::ErrorResponse> {
        if let Some(fun) = self.functions.iter_mut().find(|fun| fun.instance_id == patch_reg.instance_id) {
            // TODO(raphaelhetzel) This should probably be changed to Port<32> in the request type.
            let output_mapping = patch_reg
                .output_mapping
                .into_iter()
                .map(|(k, v)| (edgeless_api_core::port::Port(heapless::String::<32>::from_str(k).unwrap()), v))
                .collect();

            fun.inner.store.data_mut().host.data_plane.patch(output_mapping).await;
        }
        Ok(())
    }
}

impl crate::invocation::InvocationAPI for WasmiFunctionInstanceWrapper {
    async fn handle(&mut self, event: edgeless_api_core::invocation::Event) -> Result<edgeless_api_core::invocation::LinkProcessingResult, ()> {
        match event.data {
            edgeless_api_core::invocation::EventData::Call(data) => {
                let ret = self.inner.call(&event.source, event.target_port.0.as_str(), &data.0).await.unwrap();
                let own_host = &mut self.inner.store.data_mut().host;
                own_host
                    .data_plane
                    .reply(own_host.instance_id, event.source, event.stream_id, ret)
                    .await
                    .map_err(|_| ())?;
            }
            edgeless_api_core::invocation::EventData::Cast(data) => {
                self.inner.cast(&event.source, event.target_port.0.as_str(), &data.0).await.unwrap();
            }
            edgeless_api_core::invocation::EventData::CallRet(_) => todo!(),
            edgeless_api_core::invocation::EventData::CallNoRet => todo!(),
            edgeless_api_core::invocation::EventData::Err => todo!(),
        }
        Ok(edgeless_api_core::invocation::LinkProcessingResult::FINAL)
    }
}

// Taken from the Base Node
struct WasmiFunctionInstance {
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

impl WasmiFunctionInstance {
    async fn instantiate(
        // _instance_id: &edgeless_api::function_instance::InstanceId,
        // _runtime_configuration: std::collections::HashMap<String, String>,
        guest_api_host: guest_api::GuestAPIHost,
        code: &[u8],
    ) -> Result<Self, FunctionInstanceError> {
        let _comfig = wasmi::Config::default();

        let engine = wasmi::Engine::default();
        let module = wasmi::Module::new(&engine, code).unwrap();
        let mut store = wasmi::Store::new(&engine, guest_api_binding::GuestAPI { host: guest_api_host });
        let mut linker = wasmi::Linker::<guest_api_binding::GuestAPI>::new(&engine);

        linker
            .define("env", "cast_raw_asm", wasmi::Func::wrap(&mut store, guest_api_binding::cast_raw))
            .map_err(|_| FunctionInstanceError::InternalError)?;
        linker
            .define("env", "cast_asm", wasmi::Func::wrap(&mut store, guest_api_binding::cast))
            .map_err(|_| FunctionInstanceError::InternalError)?;
        linker
            .define("env", "call_raw_asm", wasmi::Func::wrap(&mut store, guest_api_binding::call_raw))
            .map_err(|_| FunctionInstanceError::InternalError)?;
        linker
            .define("env", "call_asm", wasmi::Func::wrap(&mut store, guest_api_binding::call))
            .map_err(|_| FunctionInstanceError::InternalError)?;
        linker
            .define(
                "env",
                "telemetry_log_asm",
                wasmi::Func::wrap(&mut store, guest_api_binding::telemetry_log),
            )
            .map_err(|_| FunctionInstanceError::InternalError)?;
        linker
            .define("env", "slf_asm", wasmi::Func::wrap(&mut store, guest_api_binding::slf))
            .map_err(|_| FunctionInstanceError::InternalError)?;
        linker
            .define("env", "delayed_cast_asm", wasmi::Func::wrap(&mut store, guest_api_binding::delayed_cast))
            .map_err(|_| FunctionInstanceError::InternalError)?;
        linker
            .define("env", "sync_asm", wasmi::Func::wrap(&mut store, guest_api_binding::sync))
            .map_err(|_| FunctionInstanceError::InternalError)?;

        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|_| FunctionInstanceError::InternalError)?
            .start(&mut store)
            .map_err(|_| FunctionInstanceError::InternalError)?;

        Ok(Self {
            edgeless_mem_alloc: instance
                .get_typed_func::<i32, i32>(&mut store, "edgeless_mem_alloc")
                .map_err(|_| FunctionInstanceError::BadCode)?,
            edgeless_mem_free: instance
                .get_typed_func::<(i32, i32), ()>(&mut store, "edgeless_mem_free")
                .map_err(|_| FunctionInstanceError::BadCode)?,
            edgeless_mem_clear: instance
                .get_typed_func::<(), ()>(&mut store, "edgeless_mem_clear")
                .map_err(|_| FunctionInstanceError::BadCode)?,
            edgefunctione_handle_call: instance
                .get_typed_func::<(i32, i32, i32, i32, i32, i32, i32, i32), i32>(&mut store, "handle_call_asm")
                .map_err(|_| FunctionInstanceError::BadCode)?,
            edgefunctione_handle_cast: instance
                .get_typed_func::<(i32, i32, i32, i32, i32, i32), ()>(&mut store, "handle_cast_asm")
                .map_err(|_| FunctionInstanceError::BadCode)?,
            edgefunctione_handle_init: instance
                .get_typed_func::<(i32, i32, i32, i32), ()>(&mut store, "handle_init_asm")
                .map_err(|_| FunctionInstanceError::BadCode)?,
            edgefunctione_handle_stop: instance
                .get_typed_func::<(), ()>(&mut store, "handle_stop_asm")
                .map_err(|_| FunctionInstanceError::BadCode)?,
            memory: instance.get_memory(&mut store, "memory").ok_or(FunctionInstanceError::BadCode)?,
            store,
        })
    }

    async fn init(&mut self, init_payload: Option<&str>, serialized_state: Option<&str>) -> Result<(), FunctionInstanceError> {
        let (init_payload_ptr, init_payload_len) = match init_payload {
            Some(payload) => {
                let len = payload.len();
                let ptr = helpers::copy_to_vm(
                    &mut self.store.as_context_mut(),
                    &self.memory,
                    &self.edgeless_mem_alloc,
                    payload.as_bytes(),
                )
                .map_err(|_| FunctionInstanceError::BadCode)?;
                (ptr, len as i32)
            }
            None => (0i32, 0i32),
        };

        let (serialized_state_ptr, serialized_state_len) = match serialized_state {
            Some(state) => {
                let len = state.len();
                let ptr = helpers::copy_to_vm(&mut self.store.as_context_mut(), &self.memory, &self.edgeless_mem_alloc, state.as_bytes())
                    .map_err(|_| FunctionInstanceError::BadCode)?;
                (ptr, len as i32)
            }
            None => (0i32, 0i32),
        };

        let ret = Ok(self
            .edgefunctione_handle_init
            .call(
                &mut self.store,
                (init_payload_ptr, init_payload_len, serialized_state_ptr, serialized_state_len),
            )
            .map_err(|_| FunctionInstanceError::InternalError)?);

        if init_payload_len > 0 {
            self.edgeless_mem_free
                .call(&mut self.store, (init_payload_ptr, init_payload_len))
                .map_err(|_| FunctionInstanceError::InternalError)?;
        }

        if serialized_state_len > 0 {
            self.edgeless_mem_free
                .call(&mut self.store, (serialized_state_ptr, serialized_state_len))
                .map_err(|_| FunctionInstanceError::InternalError)?;
        }

        ret
    }

    async fn cast(&mut self, src: &edgeless_api_core::instance_id::InstanceId, port: &str, msg: &[u8]) -> Result<(), FunctionInstanceError> {
        // Depending on the Function, we might employ a basic arena/bump allocator that we must reset at the end of a transaction.
        // This might be a noop if the function defines a working version of `edgeless_mem_free`.
        self.edgeless_mem_clear
            .call(&mut self.store, ())
            .map_err(|_| FunctionInstanceError::BadCode)?;

        let component_id_ptr = helpers::copy_to_vm(
            &mut self.store.as_context_mut(),
            &self.memory,
            &self.edgeless_mem_alloc,
            src.function_id.as_bytes(),
        )
        .map_err(|_| FunctionInstanceError::BadCode)?;
        let node_id_ptr = helpers::copy_to_vm(
            &mut self.store.as_context_mut(),
            &self.memory,
            &self.edgeless_mem_alloc,
            src.node_id.as_bytes(),
        )
        .map_err(|_| FunctionInstanceError::BadCode)?;

        let port_len = port.len();
        let port_ptr = helpers::copy_to_vm(&mut self.store.as_context_mut(), &self.memory, &self.edgeless_mem_alloc, port.as_bytes())
            .map_err(|_| FunctionInstanceError::BadCode)?;

        let payload_len = msg.len();
        let payload_ptr = helpers::copy_to_vm(&mut self.store.as_context_mut(), &self.memory, &self.edgeless_mem_alloc, msg)
            .map_err(|_| FunctionInstanceError::BadCode)?;

        let ret = Ok(self
            .edgefunctione_handle_cast
            .call(
                &mut self.store,
                (node_id_ptr, component_id_ptr, port_ptr, port_len as i32, payload_ptr, payload_len as i32),
            )
            .map_err(|_| FunctionInstanceError::BadCode)?);

        self.edgeless_mem_free
            .call(&mut self.store, (component_id_ptr, 16))
            .map_err(|_| FunctionInstanceError::InternalError)?;
        self.edgeless_mem_free
            .call(&mut self.store, (node_id_ptr, 16))
            .map_err(|_| FunctionInstanceError::InternalError)?;
        if payload_len > 0 {
            self.edgeless_mem_free
                .call(&mut self.store, (payload_ptr, payload_len as i32))
                .map_err(|_| FunctionInstanceError::InternalError)?;
        }
        ret
    }

    async fn call(
        &mut self,
        src: &edgeless_api_core::instance_id::InstanceId,
        port: &str,
        msg: &[u8],
    ) -> Result<crate::dataplane::CallRet, FunctionInstanceError> {
        self.edgeless_mem_clear
            .call(&mut self.store, ())
            .map_err(|_| FunctionInstanceError::BadCode)?;

        let component_id_ptr = helpers::copy_to_vm(
            &mut self.store.as_context_mut(),
            &self.memory,
            &self.edgeless_mem_alloc,
            src.function_id.as_bytes(),
        )
        .map_err(|_| FunctionInstanceError::BadCode)?;

        let node_id_ptr = helpers::copy_to_vm(
            &mut self.store.as_context_mut(),
            &self.memory,
            &self.edgeless_mem_alloc,
            src.node_id.as_bytes(),
        )
        .map_err(|_| FunctionInstanceError::BadCode)?;

        let port_len = port.len();
        let port_ptr = helpers::copy_to_vm(&mut self.store.as_context_mut(), &self.memory, &self.edgeless_mem_alloc, port.as_bytes())
            .map_err(|_| FunctionInstanceError::BadCode)?;

        let payload_len = msg.len();
        let payload_ptr = helpers::copy_to_vm(&mut self.store.as_context_mut(), &self.memory, &self.edgeless_mem_alloc, msg)
            .map_err(|_| FunctionInstanceError::BadCode)?;

        let out_ptr_ptr = self
            .edgeless_mem_alloc
            .call(&mut self.store, 4)
            .map_err(|_| FunctionInstanceError::BadCode)?;

        let out_len_ptr = self
            .edgeless_mem_alloc
            .call(&mut self.store, 4)
            .map_err(|_| FunctionInstanceError::BadCode)?;

        let callret_type = self
            .edgefunctione_handle_call
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
            .map_err(|_| FunctionInstanceError::BadCode)?;

        let ret = match callret_type {
            0 => Ok(crate::dataplane::CallRet::NoReply),
            1 => {
                // load the output pointer (inside the WASM memory) (layer of indirection to work around only using one return param)
                let out_ptr: [u8; 4] = self.memory.data_mut(&mut self.store)[out_ptr_ptr as usize..(out_ptr_ptr as usize) + 4]
                    .try_into()
                    .map_err(|_| FunctionInstanceError::InternalError)?;
                let out_ptr = i32::from_le_bytes(out_ptr);

                // load the output lenght (layer of indirection to work around only using one return param)
                let out_len: [u8; 4] = self.memory.data_mut(&mut self.store)[out_len_ptr as usize..(out_len_ptr as usize) + 4]
                    .try_into()
                    .map_err(|_| FunctionInstanceError::InternalError)?;
                let out_len = i32::from_le_bytes(out_len);

                // load the atual output param
                let out_raw = heapless::Vec::<u8, 1500>::from_slice(
                    &self.memory.data_mut(&mut self.store)[out_ptr as usize..(out_ptr as usize) + out_len as usize],
                )
                .unwrap();
                Ok(crate::dataplane::CallRet::Reply(out_raw))
            }
            _ => Ok(crate::dataplane::CallRet::Err),
        };

        self.edgeless_mem_free
            .call(&mut self.store, (component_id_ptr, 16))
            .map_err(|_| FunctionInstanceError::InternalError)?;
        self.edgeless_mem_free
            .call(&mut self.store, (node_id_ptr, 16))
            .map_err(|_| FunctionInstanceError::InternalError)?;
        // We don't need to free the data referred to by this pointer as we assume them to be stack-allocated.
        self.edgeless_mem_free
            .call(&mut self.store, (out_ptr_ptr, 4))
            .map_err(|_| FunctionInstanceError::InternalError)?;
        // We don't need to free the data referred to by this pointer as we assume them to be stack-allocated.
        self.edgeless_mem_free
            .call(&mut self.store, (out_len_ptr, 4))
            .map_err(|_| FunctionInstanceError::InternalError)?;

        if payload_len > 0 {
            self.edgeless_mem_free
                .call(&mut self.store, (payload_ptr, payload_len as i32))
                .map_err(|_| FunctionInstanceError::InternalError)?;
        }

        ret
    }

    async fn stop(&mut self) -> Result<(), FunctionInstanceError> {
        self.edgeless_mem_clear
            .call(&mut self.store, ())
            .map_err(|_| FunctionInstanceError::BadCode)?;
        self.edgefunctione_handle_stop
            .call(&mut self.store, ())
            .map_err(|_| FunctionInstanceError::BadCode)
    }
}
