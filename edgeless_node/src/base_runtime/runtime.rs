// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
/// Generic function runtime hosting a set of runners of one type (e.g. WASM/Docker)
/// Split into the active component `RuntimeTask` and the cloneable `RuntimeClient` allowing to interact with the runtime.
use futures::{SinkExt, StreamExt};

#[derive(Clone)]
pub struct RuntimeClient {
    sender: futures::channel::mpsc::UnboundedSender<RuntimeRequest>,
}

pub struct RuntimeTask<FunctionInstanceType, FunctionInstanceRunner: super::FunctionInstanceRunner<FunctionInstanceType>> {
    receiver: futures::channel::mpsc::UnboundedReceiver<RuntimeRequest>,
    data_plane_provider: edgeless_dataplane::handle::DataplaneProvider,
    state_manager: Box<dyn crate::state_management::StateManagerAPI>,
    telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
    slf_channel: futures::channel::mpsc::UnboundedSender<RuntimeRequest>,
    functions: std::collections::HashMap<edgeless_api::function_instance::InstanceId, FunctionInstanceRunner>,
    _pd: std::marker::PhantomData<FunctionInstanceType>,
}

pub enum RuntimeRequest {
    Start(Box<edgeless_api::function_instance::SpawnFunctionRequest>),
    Stop(edgeless_api::function_instance::InstanceId),
    Patch(edgeless_api::common::PatchRequest),
    FunctionExit(edgeless_api::function_instance::InstanceId, Result<(), super::FunctionInstanceError>),
}

/// Entrypoint for all runtimes based on the base_runtime.
pub fn create<FunctionInstanceType, FunctionInstanceRunner: super::FunctionInstanceRunner<FunctionInstanceType>>(
    data_plane_provider: edgeless_dataplane::handle::DataplaneProvider,
    state_manager: Box<dyn crate::state_management::StateManagerAPI>,
    telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
) -> (RuntimeClient, RuntimeTask<FunctionInstanceType, FunctionInstanceRunner>) {
    let (sender, receiver) = futures::channel::mpsc::unbounded();
    let task: RuntimeTask<FunctionInstanceType, FunctionInstanceRunner> =
        RuntimeTask::new(receiver, data_plane_provider, state_manager, telemetry_handle, sender.clone());

    let client = RuntimeClient::new(sender);

    (client, task)
}

impl<FunctionInstanceType, FunctionInstanceRunner: super::FunctionInstanceRunner<FunctionInstanceType>>
    RuntimeTask<FunctionInstanceType, FunctionInstanceRunner>
{
    fn new(
        receiver: futures::channel::mpsc::UnboundedReceiver<RuntimeRequest>,
        data_plane_provider: edgeless_dataplane::handle::DataplaneProvider,
        state_manager: Box<dyn crate::state_management::StateManagerAPI>,
        telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
        slf_channel: futures::channel::mpsc::UnboundedSender<RuntimeRequest>,
    ) -> Self {
        Self {
            receiver,
            data_plane_provider,
            state_manager,
            telemetry_handle,
            slf_channel,
            functions: std::collections::HashMap::new(),
            _pd: std::marker::PhantomData {},
        }
    }

    pub async fn run(&mut self) {
        tracing::info!("Starting Edgeless Runner Task");
        while let Some(req) = self.receiver.next().await {
            match req {
                RuntimeRequest::Start(spawn_request) => {
                    self.start_function(*spawn_request).await;
                }
                RuntimeRequest::Stop(instance_id) => {
                    self.stop_function(instance_id).await;
                }
                RuntimeRequest::Patch(update_request) => {
                    self.patch_function_links(update_request).await;
                }
                RuntimeRequest::FunctionExit(id, status) => {
                    self.function_exit(id, status).await;
                }
            }
        }
    }

    async fn start_function(&mut self, spawn_request: edgeless_api::function_instance::SpawnFunctionRequest) {
        tracing::info!(
            "Start Actor Class: {:?}; Instance ID: {:?}; Dialect: {:?}; Mapped Output {:?}; Mapped Inputs {:?}",
            spawn_request.code.function_class_id,
            spawn_request.instance_id,
            spawn_request.code.function_class_type,
            spawn_request.output_mapping.keys().map(|k| &k.0).collect::<Vec<_>>(),
            spawn_request.input_mapping.keys().map(|k| &k.0).collect::<Vec<_>>()
        );
        let instance_id = spawn_request.instance_id;
        let cloned_req = spawn_request.clone();
        let telemetry_handle = self.telemetry_handle.fork(std::collections::BTreeMap::from([(
            "FUNCTION_ID".to_string(),
            instance_id.function_id.to_string(),
        )]));
        let mut data_plane = self.data_plane_provider.get_handle_for(instance_id, Some(telemetry_handle.clone())).await;
        data_plane.update_mapping(spawn_request.input_mapping, spawn_request.output_mapping).await;
        let instance = FunctionInstanceRunner::new(
            cloned_req,
            data_plane,
            self.slf_channel.clone(),
            self.state_manager
                .get_handle(spawn_request.state_specification.state_policy, spawn_request.state_specification.state_id)
                .await,
            telemetry_handle,
        )
        .await;
        self.functions.insert(instance_id, instance);
    }

    async fn stop_function(&mut self, instance_id: edgeless_api::function_instance::InstanceId) {
        tracing::info!("Stop Actor {instance_id:?}");
        if let Some(instance) = self.functions.get_mut(&instance_id) {
            instance.stop().await;
        }
    }

    async fn patch_function_links(&mut self, update_request: edgeless_api::common::PatchRequest) {
        tracing::info!("Patch Actor {:?}", update_request.function_id);
        if let Some(instance) = self.functions.get_mut(&update_request.function_id) {
            instance.patch(update_request).await;
        }
    }

    async fn function_exit(&mut self, instance_id: edgeless_api::function_instance::InstanceId, status: Result<(), super::FunctionInstanceError>) {
        tracing::info!("Function Exit Event: {instance_id:?} {status:?}");
        self.functions.remove(&instance_id);
    }
}

impl RuntimeClient {
    pub fn new(runtime_request_sender: futures::channel::mpsc::UnboundedSender<RuntimeRequest>) -> Self {
        RuntimeClient {
            sender: runtime_request_sender,
        }
    }
}

#[async_trait::async_trait]
impl super::RuntimeAPI for RuntimeClient {
    async fn start(&mut self, request: edgeless_api::function_instance::SpawnFunctionRequest) -> anyhow::Result<()> {
        match self.sender.send(RuntimeRequest::Start(Box::new(request))).await {
            Ok(_) => Ok(()),
            Err(_) => Err(anyhow::anyhow!("Runner Channel Error")),
        }
    }

    async fn stop(&mut self, instance_id: edgeless_api::function_instance::InstanceId) -> anyhow::Result<()> {
        match self.sender.send(RuntimeRequest::Stop(instance_id)).await {
            Ok(_) => Ok(()),
            Err(_) => Err(anyhow::anyhow!("Runner Channel Error")),
        }
    }

    async fn patch(&mut self, update: edgeless_api::common::PatchRequest) -> anyhow::Result<()> {
        match self.sender.send(RuntimeRequest::Patch(update)).await {
            Ok(_) => Ok(()),
            Err(_) => Err(anyhow::anyhow!("Runner Channel Error")),
        }
    }
}
