pub(crate) struct HostApiImpl {
    pub(crate) host: crate::base_runtime::guest_api::GuestAPIHost,
    pub(crate) alloc: talc::Talck<spin::Mutex<()>, talc::ErrOnOom>,
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
