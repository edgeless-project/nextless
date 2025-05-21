// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
pub mod alias_mapping;
pub mod function_instance_runner;
pub mod function_instance_runner_common;
pub mod function_instance_runner_thread;
pub mod guest_api;
pub mod runtime;

/// (Deprecated) Trait to be implemented by each runtime.
/// You won't need to implement this trait if you use the base_runtime that is generic over the `FunctionInstance` trait
#[async_trait::async_trait]
pub trait RuntimeAPI {
    async fn start(&mut self, request: edgeless_api::function_instance::SpawnFunctionRequest) -> anyhow::Result<()>;
    async fn stop(&mut self, instance_id: edgeless_api::function_instance::InstanceId) -> anyhow::Result<()>;
    async fn patch(&mut self, update: edgeless_api::common::PatchRequest) -> anyhow::Result<()>;
}

#[async_trait::async_trait]
pub trait FunctionInstanceRunner<Instance> {
    async fn new(
        spawn_req: edgeless_api::function_instance::SpawnFunctionRequest,
        data_plane: edgeless_dataplane::handle::DataplaneHandle,
        runtime_api: futures::channel::mpsc::UnboundedSender<runtime::RuntimeRequest>,
        state_handle: Box<dyn crate::state_management::StateHandleAPI>,
        telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
    ) -> Self;
    async fn stop(&mut self);
    async fn patch(&mut self, update_request: edgeless_api::common::PatchRequest);
}

/// This must be implemented for each virtualization technology.
/// As suggested by the name, it contains a single instance of a function.
#[async_trait::async_trait]
pub trait FunctionInstance: Send + 'static {
    async fn instantiate(
        instance_id: &edgeless_api::function_instance::InstanceId,
        runtime_configuration: std::collections::HashMap<String, String>,
        guest_api_host: crate::base_runtime::guest_api::GuestAPIHost,
        code: &[u8],
    ) -> FunctionInstanceResult<Box<Self>>;
    async fn init(&mut self, init_payload: Option<&str>, serialized_state: Option<&[u8]>) -> FunctionInstanceResult<()>;
    async fn cast(&mut self, src: &edgeless_api::function_instance::InstanceId, port: &str, msg: &[u8]) -> FunctionInstanceResult<()>;
    async fn call(
        &mut self,
        src: &edgeless_api::function_instance::InstanceId,
        port: &str,
        msg: &[u8],
    ) -> FunctionInstanceResult<edgeless_dataplane::core::CallRet>;
    async fn stop(&mut self) -> FunctionInstanceResult<()>;
}

pub trait FunctionInstanceSync: Send {
    fn instantiate(
        instance_id: &edgeless_api::function_instance::InstanceId,
        runtime_configuration: std::collections::HashMap<String, String>,
        guest_api_host: crate::base_runtime::guest_api::GuestAPIHost,
        code: &[u8],
    ) -> FunctionInstanceResult<Box<Self>>;
    fn init(&mut self, init_payload: Option<&str>, serialized_state: Option<&[u8]>) -> FunctionInstanceResult<()>;
    fn cast(&mut self, src: &edgeless_api::function_instance::InstanceId, port: &str, msg: &[u8]) -> FunctionInstanceResult<()>;
    fn call(
        &mut self,
        src: &edgeless_api::function_instance::InstanceId,
        port: &str,
        msg: &[u8],
    ) -> FunctionInstanceResult<edgeless_dataplane::core::CallRet>;
    fn stop(&mut self) -> FunctionInstanceResult<()>;
}

pub type FunctionInstanceResult<T> = Result<T, FunctionInstanceError>;

#[derive(thiserror::Error, Debug)]
pub enum FunctionInstanceError {
    #[error("Bad Code: {0}")]
    BadCode(#[source] anyhow::Error),
    #[error("Internal Error: {0}")]
    Internal(#[source] anyhow::Error),
}
