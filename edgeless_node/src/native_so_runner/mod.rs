// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 University of Cambridge, System Research Group
// SPDX-FileCopyrightText: © 2024 Roman Kolcun <roman.kolcun@cl.cam.ac.uk>
// SPDX-License-Identifier: MIT

use std::io::Write;

pub struct NativeFunctionInstance {
    lib: libloading::Library,
    _host_api: Box<HostApiImpl>,
}

struct HostApiImpl {
    host: crate::base_runtime::guest_api::GuestAPIHost,
    alloc: talc::Talck<spin::Mutex<()>, talc::ErrOnOom>,
}

impl<'a> edgeless_actor_abi::HostApi<'a> for HostApiImpl {
    fn cast(&mut self, output_port: edgeless_actor_abi::Port, msg: edgeless_actor_abi::Message) -> edgeless_actor_abi::HostResult<()> {
        self.host
            .handle
            .clone()
            .block_on(self.host.cast_alias(output_port.0, msg.0))
            .map_err(|_| edgeless_actor_abi::HostError::BadAlias)
    }

    fn cast_raw(
        &mut self,
        dst_id: edgeless_actor_abi::ActorId,
        dst_input_port: edgeless_actor_abi::Port,
        msg: edgeless_actor_abi::Message,
    ) -> edgeless_actor_abi::HostResult<()> {
        self.host
            .handle
            .clone()
            .block_on(self.host.cast_raw(
                edgeless_api::function_instance::InstanceId {
                    node_id: uuid::Uuid::from_bytes(dst_id.node_id),
                    function_id: uuid::Uuid::from_bytes(dst_id.node_id),
                },
                edgeless_api::function_instance::PortId(dst_input_port.0.to_string()),
                msg.0,
            ))
            .map_err(|_| edgeless_actor_abi::HostError::Internal)
    }

    fn delayed_cast(
        &mut self,
        delay_ms: u64,
        output_port: edgeless_actor_abi::Port,
        msg: edgeless_actor_abi::Message,
    ) -> edgeless_actor_abi::HostResult<()> {
        self.host
            .handle
            .clone()
            .block_on(self.host.delayed_cast(delay_ms, output_port.0, msg.0))
            .map_err(|_| edgeless_actor_abi::HostError::BadAlias)
    }

    fn call_raw(
        &'a mut self,
        dst_id: edgeless_actor_abi::ActorId,
        dst_port: edgeless_actor_abi::Port,
        msg: edgeless_actor_abi::Message,
    ) -> edgeless_actor_abi::HostResult<edgeless_actor_abi::CallRet<'a>> {
        self.host
            .handle
            .clone()
            .block_on(self.host.call_raw(
                edgeless_api::function_instance::InstanceId {
                    node_id: uuid::Uuid::from_bytes(dst_id.node_id),
                    function_id: uuid::Uuid::from_bytes(dst_id.node_id),
                },
                edgeless_api::function_instance::PortId(dst_port.0.to_string()),
                msg.0,
            ))
            .map_err(|_| edgeless_actor_abi::HostError::Internal)
            .map(|r| match r {
                edgeless_dataplane::core::CallRet::NoReply => edgeless_actor_abi::CallRet::NoReply,
                edgeless_dataplane::core::CallRet::Reply(data) => {
                    let mut v = allocator_api2::vec::Vec::new_in(&self.alloc as &dyn allocator_api2::alloc::Allocator);
                    v.extend_from_slice(&data[..]);
                    edgeless_actor_abi::CallRet::Reply(v)
                }
                edgeless_dataplane::core::CallRet::Err => edgeless_actor_abi::CallRet::Err,
            })
    }

    fn call(
        &'a mut self,
        output_port: edgeless_actor_abi::Port,
        msg: edgeless_actor_abi::Message,
    ) -> edgeless_actor_abi::HostResult<edgeless_actor_abi::CallRet<'a>> {
        self.host
            .handle
            .clone()
            .block_on(self.host.call_alias(output_port.0, msg.0))
            .map_err(|_| edgeless_actor_abi::HostError::Internal)
            .map(|r| match r {
                edgeless_dataplane::core::CallRet::NoReply => edgeless_actor_abi::CallRet::NoReply,
                edgeless_dataplane::core::CallRet::Reply(data) => {
                    let mut v = allocator_api2::vec::Vec::new_in(&self.alloc as &dyn allocator_api2::alloc::Allocator);
                    v.extend_from_slice(&data[..]);
                    edgeless_actor_abi::CallRet::Reply(v)
                }
                edgeless_dataplane::core::CallRet::Err => edgeless_actor_abi::CallRet::Err,
            })
    }

    fn telemetry_log(&mut self, level: edgeless_actor_abi::LogLevel, component: &str, msg: &str) -> edgeless_actor_abi::HostResult<()> {
        self.host.handle.clone().block_on(self.host.telemetry_log(
            match level {
                edgeless_actor_abi::LogLevel::Error => edgeless_telemetry::telemetry_events::TelemetryLogLevel::Error,
                edgeless_actor_abi::LogLevel::Warn => edgeless_telemetry::telemetry_events::TelemetryLogLevel::Warn,
                edgeless_actor_abi::LogLevel::Info => edgeless_telemetry::telemetry_events::TelemetryLogLevel::Warn,
                edgeless_actor_abi::LogLevel::Debug => edgeless_telemetry::telemetry_events::TelemetryLogLevel::Debug,
                edgeless_actor_abi::LogLevel::Trace => edgeless_telemetry::telemetry_events::TelemetryLogLevel::Trace,
            },
            component,
            msg,
        ));
        Ok(())
    }

    fn slf(&mut self) -> edgeless_actor_abi::HostResult<edgeless_actor_abi::ActorId> {
        let id = self.host.handle.clone().block_on(self.host.slf());
        Ok(edgeless_actor_abi::ActorId {
            node_id: id.node_id.into_bytes(),
            component_id: id.function_id.into_bytes(),
        })
    }

    fn state_sync(&mut self, _state: &[u8]) -> edgeless_actor_abi::HostResult<()> {
        // Not yet implemented for &[u8]
        Err(edgeless_actor_abi::HostError::Internal)
    }

    fn allocator(&mut self) -> &dyn allocator_api2::alloc::Allocator {
        &self.alloc
    }
}

impl crate::base_runtime::FunctionInstanceSync for NativeFunctionInstance {
    fn instantiate(
        _instance_id: &edgeless_api::function_instance::InstanceId,
        _runtime_configuration: std::collections::HashMap<String, String>,
        guest_api_host: crate::base_runtime::guest_api::GuestAPIHost,
        code: &[u8],
    ) -> crate::base_runtime::FunctionInstanceResult<Box<Self>> {
        let mut code_store = tempfile::NamedTempFile::new().map_err(|e| crate::base_runtime::FunctionInstanceError::Internal(e.into()))?;
        code_store
            .write_all(code)
            .map_err(|e| crate::base_runtime::FunctionInstanceError::Internal(e.into()))?;
        unsafe {
            let buffer = Box::<[u8; 128 * 1024 * 1024]>::new_uninit();
            let mut buffer = buffer.assume_init();
            let mut alloc = talc::Talc::new(talc::ErrOnOom);
            alloc
                .claim(talc::Span::from_array(&mut *buffer))
                .map_err(|_| crate::base_runtime::FunctionInstanceError::Internal(anyhow::anyhow!("Could not claim Memory")))?;

            let start = std::time::Instant::now();
            let lib = libloading::Library::new(code_store.path()).map_err(|e| crate::base_runtime::FunctionInstanceError::BadCode(e.into()))?;
            log::info!("Instantiation Time: {:?}", start.elapsed());

            let host_api: libloading::Symbol<*mut &mut dyn edgeless_actor_abi::HostApi> = lib
                .get(b"HOST_API")
                .map_err(|e| crate::base_runtime::FunctionInstanceError::Internal(e.into()))?;

            let mut api = Box::new(HostApiImpl {
                host: guest_api_host,
                alloc: alloc.lock::<spin::Mutex<()>>(),
                // alloc: MockAlloc {},
            });

            **host_api = api.as_mut();

            Ok(Box::new(Self { lib, _host_api: api }))
        }
    }

    fn init(&mut self, init_payload: Option<&str>, serialized_state: Option<&[u8]>) -> crate::base_runtime::FunctionInstanceResult<()> {
        unsafe {
            let init_method: libloading::Symbol<edgeless_actor_abi::HandleInit> = self
                .lib
                .get(b"handle_init")
                .map_err(|e| crate::base_runtime::FunctionInstanceError::Internal(e.into()))?;
            (*init_method)(init_payload.map(|s| s.as_bytes()), serialized_state)
                .map_err(|e| crate::base_runtime::FunctionInstanceError::BadCode(e.into()))
        }
    }

    fn cast(&mut self, src: &edgeless_api::function_instance::InstanceId, port: &str, msg: &[u8]) -> crate::base_runtime::FunctionInstanceResult<()> {
        unsafe {
            let handle_cast_method: libloading::Symbol<edgeless_actor_abi::HandleCast> = self
                .lib
                .get(b"handle_cast")
                .map_err(|e| crate::base_runtime::FunctionInstanceError::Internal(e.into()))?;
            (*handle_cast_method)(
                edgeless_actor_abi::ActorId {
                    node_id: src.node_id.into_bytes(),
                    component_id: src.function_id.into_bytes(),
                },
                port,
                msg,
            )
            .map_err(|e| crate::base_runtime::FunctionInstanceError::BadCode(e.into()))
        }
    }

    fn call<'a>(
        &'a mut self,
        src: &edgeless_api::function_instance::InstanceId,
        port: &str,
        msg: &[u8],
    ) -> crate::base_runtime::FunctionInstanceResult<edgeless_dataplane::core::CallRet> {
        unsafe {
            let handle_call_method: libloading::Symbol<edgeless_actor_abi::HandleCall<'a>> = self
                .lib
                .get(b"handle_call")
                .map_err(|e| crate::base_runtime::FunctionInstanceError::Internal(e.into()))?;
            (*handle_call_method)(
                edgeless_actor_abi::ActorId {
                    node_id: src.node_id.into_bytes(),
                    component_id: src.function_id.into_bytes(),
                },
                port,
                msg,
            )
            .map(|ret| match ret {
                edgeless_actor_abi::CallRet::NoReply => edgeless_dataplane::core::CallRet::NoReply,
                edgeless_actor_abi::CallRet::Reply(data) => edgeless_dataplane::core::CallRet::Reply(Vec::from(data.as_slice())),
                edgeless_actor_abi::CallRet::Err => edgeless_dataplane::core::CallRet::Err,
            })
            .map_err(|e| crate::base_runtime::FunctionInstanceError::BadCode(e.into()))
        }
    }

    fn stop(&mut self) -> crate::base_runtime::FunctionInstanceResult<()> {
        unsafe {
            let handle_stop_method: libloading::Symbol<edgeless_actor_abi::HandleStop> = self
                .lib
                .get(b"handle_stop")
                .map_err(|e| crate::base_runtime::FunctionInstanceError::Internal(e.into()))?;
            (*handle_stop_method)().map_err(|e| crate::base_runtime::FunctionInstanceError::BadCode(e.into()))
        }
    }
}
