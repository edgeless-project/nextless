// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

/// Each function instance can import a set of functions that need to be implemented on the host-side.
/// This provides the generic host-side implementation of these functions.
/// Those need to be made available to the guest using a virtualization-specific interface/binding.
pub struct GuestAPIHost {
    pub instance_id: edgeless_api_core::instance_id::InstanceId,
    pub data_plane: crate::dataplane::EmbeddedDataplaneHandle,
    // pub state_handle: Box<dyn crate::state_management::StateHandleAPI>,
    // pub telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
    // pub poison_pill_receiver: tokio::sync::broadcast::Receiver<()>,
    // pub tracing_context: std::sync::Arc<tokio::sync::Mutex<super::function_instance_runner::TracingContext>>,
}

/// Errors to be reported by the host side of the guest binding.
/// This may need to be bridged into the runtime by the virtualization-specific runtime implementation.
#[derive(Debug)]
pub enum GuestAPIError {
    UnknownAlias,
    Unimplemented,
}

impl GuestAPIHost {
    pub async fn cast_alias(&mut self, alias: &str, msg: &[u8]) -> Result<(), GuestAPIError> {
        self.data_plane
            .send_alias(
                alias,
                msg,
                // alias.to_string(),
                // msg.to_string(),
                // self.tracing_context.lock().await.parent_context.clone(),
            )
            .await;
        // .map_err(|_e| GuestAPIError::UnknownAlias)
        Ok(())
    }

    pub async fn cast_raw(
        &mut self,
        target: edgeless_api_core::instance_id::InstanceId,
        target_port: edgeless_api_core::port::Port<32>,
        msg: &str,
    ) -> Result<(), GuestAPIError> {
        self.data_plane
            .send(
                self.instance_id.clone(),
                target,
                target_port,
                msg.as_bytes(),
                // self.tracing_context.lock().await.parent_context.clone(),
            )
            .await;
        Ok(())
    }

    pub async fn call_alias(&mut self, alias: &str, msg: &str) -> Result<crate::dataplane::CallRet, GuestAPIError> {
        // futures::select! {
        //     _ = Box::pin(self.poison_pill_receiver.recv()).fuse() => {
        //         Ok(edgeless_dataplane::core::CallRet::Err)
        //     },
        //     call_res = Box::pin(self.data_plane.call_alias(alias.to_string(), msg.to_string(), self.tracing_context.lock().await.parent_context.clone()).fuse()) => {
        //         Ok(call_res)
        //     }
        // }
        Err(GuestAPIError::Unimplemented)
    }

    pub async fn call_raw(
        &mut self,
        target: edgeless_api_core::instance_id::InstanceId,
        target_port: edgeless_api_core::port::Port<32>,
        msg: &str,
    ) -> Result<crate::dataplane::CallRet, GuestAPIError> {
        // futures::select! {
        //     _ = Box::pin(self.poison_pill_receiver.recv()).fuse() => {
        //         Ok(edgeless_dataplane::core::CallRet::Err)
        //     },
        //     call_res = Box::pin(self.data_plane.call(target, target_port, msg.to_string(), self.tracing_context.lock().await.parent_context.clone())).fuse() => {
        //         Ok(call_res)
        //     }
        // }
        Err(GuestAPIError::Unimplemented)
    }

    pub async fn telemetry_log(&mut self, lvl: super::TelemetryLogLevel, target: &str, msg: &str) {
        // self.telemetry_handle.observe(
        //     edgeless_telemetry::telemetry_events::TelemetryEvent::FunctionLogEntry(lvl, target.to_string(), msg.to_string()),
        //     std::collections::BTreeMap::new(),
        // );
        log::info!("Function Log: {}", msg);
    }

    pub async fn slf(&mut self) -> edgeless_api_core::instance_id::InstanceId {
        self.instance_id
    }

    pub async fn delayed_cast(&mut self, delay: u64, target_alias: &str, payload: &str) -> Result<(), GuestAPIError> {
        // let mut cloned_plane = self.data_plane.clone();
        // let cloned_msg = payload.to_string();
        // let cloned_alias = target_alias.to_string();

        // // let cloned_context = self.tracing_context.lock().await.parent_context.clone();
        // let cloned_tracer = self.tracing_context.lock().await.tracer.clone();

        // tokio::spawn(async move {
        //     let span = cloned_tracer.start("wait");
        //     tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
        //     cloned_plane
        //         .send_alias(cloned_alias, cloned_msg, opentelemetry::Context::new())
        //         .await
        //         .unwrap();
        // });

        Err(GuestAPIError::Unimplemented)
    }

    pub async fn sync(&mut self, serialized_state: &str) -> Result<(), GuestAPIError> {
        // self.state_handle.set(serialized_state.to_string()).await;
        // log::info!("Function State Sync: {}", serialized_state);
        Err(GuestAPIError::Unimplemented)
    }
}
