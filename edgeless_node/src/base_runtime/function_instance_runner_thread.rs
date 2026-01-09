// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2024 Yahya Arakil
// SPDX-License-Identifier: MIT
use futures::{FutureExt, SinkExt};
use opentelemetry::trace::TraceContextExt;
use std::marker::PhantomData;
use tracing_opentelemetry::OpenTelemetrySpanExt;

use super::{FunctionInstanceError, FunctionInstanceSync};

/// This is the main interface for executing/managing a function instance.
/// Owning client for a single function instance task.
/// It is generic over the runtime technology (e.g. WASM).
/// FunctionInstanceRunner (with it's FunctionInstanceTask) do most of the heavy lifting/lifetime management,
/// while the technology specific implementations implement `FunctionInstance` interact and bind a virtualization technology.
pub struct FunctionInstanceRunner<FunctionInstanceType: FunctionInstanceSync> {
    task_handle: Option<std::thread::JoinHandle<()>>,
    data_plane: edgeless_dataplane::handle::DataplaneHandle,
    poison_pill_sender: tokio::sync::broadcast::Sender<()>,
    _instance: PhantomData<FunctionInstanceType>,
}

/// This is a runnable object (with all required state) actually executing a function.
/// It is managed/owned by a FunctionInstanceRunner, which also runs it using a tokio task.
struct FunctionInstanceTask<FunctionInstanceType: FunctionInstanceSync> {
    function_instance: Option<Box<FunctionInstanceType>>,

    poison_pill_receiver: tokio::sync::broadcast::Receiver<()>,
    guest_api_host: Option<super::guest_api::GuestAPIHost>,
    telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
    data_plane: edgeless_dataplane::handle::DataplaneHandle,
    runtime_api: futures::channel::mpsc::UnboundedSender<super::runtime::RuntimeRequest>,
    tracing_context: std::sync::Arc<tokio::sync::Mutex<super::function_instance_runner_common::TracingContext>>,

    instance_id: edgeless_api::function_instance::InstanceId,
    duration_soft_limit: std::time::Duration,
    init_payload: Option<String>,
    serialized_state: Option<String>,
    code: Vec<u8>,
}

#[async_trait::async_trait]
impl<FunctionInstanceType: super::FunctionInstanceSync + 'static> super::FunctionInstanceRunner<FunctionInstanceType>
    for FunctionInstanceRunner<FunctionInstanceType>
{
    async fn new(
        spawn_req: edgeless_api::function_instance::SpawnFunctionRequest,
        data_plane: edgeless_dataplane::handle::DataplaneHandle,
        runtime_api: futures::channel::mpsc::UnboundedSender<super::runtime::RuntimeRequest>,
        state_handle: Box<dyn crate::state_management::StateHandleAPI>,
        telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
    ) -> Self {
        let instance_id = spawn_req.instance_id;
        let mut telemetry_handle = telemetry_handle;
        let mut state_handle = state_handle;
        let data_plane = data_plane;

        let (poison_pill_sender, poison_pill_receiver) = tokio::sync::broadcast::channel::<()>(1);
        let serialized_state = state_handle.get().await;

        let tracing_context = std::sync::Arc::new(tokio::sync::Mutex::new(super::function_instance_runner_common::TracingContext {
            parent_context: opentelemetry::Context::new(),
        }));

        let guest_api_host = crate::base_runtime::guest_api::GuestAPIHost {
            instance_id,
            data_plane: data_plane.clone(),
            state_handle,
            telemetry_handle: telemetry_handle.fork(std::collections::BTreeMap::new()),
            poison_pill_receiver: poison_pill_sender.subscribe(),
            tracing_context: tracing_context.clone(),
            handle: tokio::runtime::Handle::current(),
        };

        let duration_soft_limit = std::time::Duration::from_millis(50);

        let task = Box::new(FunctionInstanceTask::<FunctionInstanceType>::new(
            poison_pill_receiver,
            telemetry_handle,
            guest_api_host,
            spawn_req.code.function_class_code.clone(),
            data_plane.clone(),
            serialized_state,
            spawn_req.annotations.get("init-payload").cloned(),
            runtime_api,
            instance_id,
            tracing_context,
            duration_soft_limit,
        ));

        let task_handle = std::thread::spawn(move || {
            let mut task = task;
            task.run();
        });

        Self {
            task_handle: Some(task_handle),
            poison_pill_sender,
            data_plane: data_plane.clone(),
            _instance: PhantomData {},
        }
    }

    async fn stop(&mut self) {
        self.poison_pill_sender.send(()).unwrap();

        if let Some(handle) = self.task_handle.take() {
            handle.join().unwrap();
        }
    }

    async fn patch(&mut self, update_request: edgeless_api::common::PatchRequest) {
        self.data_plane
            .update_mapping(update_request.input_mapping, update_request.output_mapping)
            .await;
    }
}

impl<FunctionInstanceType: FunctionInstanceSync> FunctionInstanceTask<FunctionInstanceType> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        poison_pill_receiver: tokio::sync::broadcast::Receiver<()>,
        telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
        guest_api_host: super::guest_api::GuestAPIHost,
        code: Vec<u8>,
        data_plane: edgeless_dataplane::handle::DataplaneHandle,
        serialized_state: Option<String>,
        init_param: Option<String>,
        runtime_api: futures::channel::mpsc::UnboundedSender<super::runtime::RuntimeRequest>,
        instance_id: edgeless_api::function_instance::InstanceId,
        tracing_context: std::sync::Arc<tokio::sync::Mutex<super::function_instance_runner_common::TracingContext>>,
        duration_soft_limit: std::time::Duration,
    ) -> Self {
        Self {
            poison_pill_receiver,
            function_instance: None,
            guest_api_host: Some(guest_api_host),
            telemetry_handle,
            code,
            data_plane,
            serialized_state,
            init_payload: init_param,
            runtime_api,
            instance_id,
            tracing_context,
            duration_soft_limit,
        }
    }

    /// Function lifecycle; Runs until the poison pill is received or there is an error.
    /// Always calls the exit handler (with the exit status)
    pub fn run(&mut self) {
        let mut res = self.instantiate();
        if res.is_ok() {
            res = self.init();
        }
        if res.is_ok() {
            res = self.processing_loop();
        }
        self.exit(res);
    }

    fn instantiate(&mut self) -> Result<(), super::FunctionInstanceError> {
        // self.data_plane.set_tracer(self.tracing_context.lock().await.tracer.clone());

        let start = tokio::time::Instant::now();
        {
            let _span = tracing::trace_span!(
                "instantiate",
                node_id = self.instance_id.node_id.to_string(),
                component_id = self.instance_id.function_id.to_string(),
            )
            .entered();

            let runtime_configuration = std::collections::HashMap::new();
            self.function_instance = Some(FunctionInstanceType::instantiate(
                &self.instance_id,
                runtime_configuration,
                self.guest_api_host
                    .take()
                    .ok_or(super::FunctionInstanceError::Internal(anyhow::anyhow!("Guest API host already taken")))?,
                &self.code,
            )?);
        }

        self.telemetry_handle.observe(
            edgeless_telemetry::telemetry_events::TelemetryEvent::FunctionInstantiate(start.elapsed()),
            std::collections::BTreeMap::new(),
        );

        Ok(())
    }

    fn init(&mut self) -> Result<(), super::FunctionInstanceError> {
        let start = tokio::time::Instant::now();
        {
            let _span = tracing::trace_span!(
                "init",
                node_id = self.instance_id.node_id.to_string(),
                component_id = self.instance_id.function_id.to_string(),
            )
            .entered();

            Self::get_function_instance(&mut self.function_instance)?
                .init(self.init_payload.as_deref(), self.serialized_state.as_ref().map(|s| s.as_bytes()))?;
        }

        self.telemetry_handle.observe(
            edgeless_telemetry::telemetry_events::TelemetryEvent::FunctionInit(start.elapsed()),
            std::collections::BTreeMap::new(),
        );

        Ok(())
    }

    fn processing_loop(&mut self) -> Result<(), super::FunctionInstanceError> {
        loop {
            futures::executor::block_on(async {
                futures::select! {
                    // Given each function instance is an independent task, the runtime needs to send a poison pill to cleanly stop it (processed here)
                    _ = Box::pin(self.poison_pill_receiver.recv()).fuse() => {
                        self.stop()
                    },
                    // Receive a normal event from the dataplane and invoke the function instance
                    edgeless_dataplane::core::DataplaneEvent{source_id, channel_id, message, target_port, source_port: _, context: span_context} =  Box::pin(self.data_plane.receive_next()).fuse() => {
                        self.process_message(
                            source_id,
                            channel_id,
                            message,
                            target_port,
                            span_context
                        )
                    }
                }
            })?;
        }
    }

    fn process_message(
        &mut self,
        source_id: edgeless_api::function_instance::InstanceId,
        channel_id: u64,
        message: edgeless_dataplane::core::Message,
        target_port: edgeless_api::function_instance::PortId,
        context: opentelemetry::trace::SpanContext,
    ) -> Result<(), super::FunctionInstanceError> {
        match message {
            edgeless_dataplane::core::Message::Cast(payload) => self.process_cast_message(source_id, target_port, &payload, context),
            edgeless_dataplane::core::Message::Call(payload) => self.process_call_message(source_id, target_port, &payload, channel_id, context),
            _ => {
                log::debug!("Unprocessed Message");
                Ok(())
            }
        }
    }

    fn process_cast_message(
        &mut self,
        source_id: edgeless_api::function_instance::InstanceId,
        target_port: edgeless_api::function_instance::PortId,
        payload: &[u8],
        span_context: opentelemetry::trace::SpanContext,
    ) -> Result<(), super::FunctionInstanceError> {
        let start = tokio::time::Instant::now();
        let span = tracing::trace_span!(
            "actor_invocation",
            target_port = target_port.0,
            node_id = self.instance_id.node_id.to_string(),
            component_id = self.instance_id.function_id.to_string(),
            invocation_type = "cast",
        );
        if !span.is_disabled() {
            let parent_context = if span_context.is_valid() {
                opentelemetry::Context::new().with_remote_span_context(span_context)
            } else {
                opentelemetry::Context::new()
            };
            span.set_parent(parent_context).unwrap();
        }
        self.tracing_context.blocking_lock().parent_context = span.context();

        let exec_result = {
            let _s = span.entered();
            Self::get_function_instance(&mut self.function_instance)?.cast(&source_id, target_port.0.as_str(), payload)
        };

        let duration = start.elapsed();

        self.tracing_context.blocking_lock().parent_context = opentelemetry::Context::new();
        self.telemetry_handle.observe(
            edgeless_telemetry::telemetry_events::TelemetryEvent::FunctionInvocationCompleted {
                duration,
                error: exec_result.is_err(),
                under_duration_soft_limit: duration < self.duration_soft_limit,
            },
            std::collections::BTreeMap::from([
                ("EVENT_TYPE".to_string(), "CAST".to_string()),
                ("PORT".to_string(), target_port.0.clone()),
            ]),
        );

        exec_result
    }

    fn process_call_message(
        &mut self,
        source_id: edgeless_api::function_instance::InstanceId,
        target_port: edgeless_api::function_instance::PortId,
        payload: &[u8],
        channel_id: u64,
        span_context: opentelemetry::trace::SpanContext,
    ) -> Result<(), super::FunctionInstanceError> {
        let start = tokio::time::Instant::now();

        let span = tracing::info_span!(
            "actor_invocation",
            target_port = target_port.0,
            node_id = self.instance_id.node_id.to_string(),
            component_id = self.instance_id.function_id.to_string(),
            invocation_type = "call",
        );

        if !span.is_disabled() {
            let parent_context = if span_context.is_valid() {
                opentelemetry::Context::new().with_remote_span_context(span_context)
            } else {
                opentelemetry::Context::new()
            };
            span.set_parent(parent_context).unwrap();
        }
        self.tracing_context.blocking_lock().parent_context = span.context();
        let res = {
            let _s = span.entered();
            Self::get_function_instance(&mut self.function_instance)?.call(&source_id, target_port.0.as_str(), payload)
        };
        let duration = start.elapsed();

        self.tracing_context.blocking_lock().parent_context = opentelemetry::Context::new();
        self.telemetry_handle.observe(
            edgeless_telemetry::telemetry_events::TelemetryEvent::FunctionInvocationCompleted {
                duration,
                error: res.is_err(),
                under_duration_soft_limit: duration < self.duration_soft_limit,
            },
            std::collections::BTreeMap::from([
                ("EVENT_TYPE".to_string(), "CALL".to_string()),
                ("PORT".to_string(), target_port.0.clone()),
            ]),
        );

        let mut wh = self.data_plane.clone();
        futures::executor::block_on(wh.reply(source_id, channel_id, res?));
        Ok(())
    }

    fn stop(&mut self) -> Result<(), super::FunctionInstanceError> {
        let start = tokio::time::Instant::now();

        Self::get_function_instance(&mut self.function_instance)?.stop()?;

        self.telemetry_handle.observe(
            edgeless_telemetry::telemetry_events::TelemetryEvent::FunctionStop(start.elapsed()),
            std::collections::BTreeMap::new(),
        );

        Ok(())
    }

    fn exit(&mut self, exit_status: Result<(), super::FunctionInstanceError>) {
        self.telemetry_handle.observe(
            edgeless_telemetry::telemetry_events::TelemetryEvent::FunctionExit(match &exit_status {
                Ok(_) => edgeless_telemetry::telemetry_events::FunctionExitStatus::Ok,
                Err(exit_err) => match exit_err {
                    FunctionInstanceError::BadCode(_) => edgeless_telemetry::telemetry_events::FunctionExitStatus::CodeError,
                    _ => edgeless_telemetry::telemetry_events::FunctionExitStatus::InternalError,
                },
            }),
            std::collections::BTreeMap::new(),
        );

        futures::executor::block_on(
            self.runtime_api
                .send(super::runtime::RuntimeRequest::FunctionExit(self.instance_id, exit_status)),
        )
        .unwrap_or_else(|_| log::error!("FunctionInstance outlived runner."));
    }

    fn get_function_instance(
        function_instance: &mut Option<Box<FunctionInstanceType>>,
    ) -> Result<&mut FunctionInstanceType, super::FunctionInstanceError> {
        function_instance
            .as_mut()
            .map(|i| i.as_mut())
            .ok_or(anyhow::anyhow!("Function Runtime: Function Instance is None."))
            .map_err(super::FunctionInstanceError::Internal)
    }
}
