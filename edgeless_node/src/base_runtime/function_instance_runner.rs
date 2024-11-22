// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
use futures::{FutureExt, SinkExt};
use opentelemetry::{trace::{TraceContextExt, Tracer}};
use opentelemetry::trace::Span;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry::trace::TracerProvider;
use std::marker::PhantomData;

use super::{FunctionInstance, FunctionInstanceError};

/// This is the main interface for executing/managing a function instance.
/// Owning client for a single function instance task.
/// It is generic over the runtime technology (e.g. WASM).
/// FunctionInstanceRunner (with it's FunctionInstanceTask) do most of the heavy lifting/lifetime management,
/// while the technology specific implementations implement `FunctionInstance` interact and bind a virtualization technology.
pub struct FunctionInstanceRunner<FunctionInstanceType: FunctionInstance> {
    task_handle: Option<tokio::task::JoinHandle<()>>,
    data_plane: edgeless_dataplane::handle::DataplaneHandle,
    poison_pill_sender: tokio::sync::broadcast::Sender<()>,
    _instance: PhantomData<FunctionInstanceType>,
}

/// This is a runnable object (with all required state) actually executing a function.
/// It is managed/owned by a FunctionInstanceRunner, which also runs it using a tokio task.
struct FunctionInstanceTask<FunctionInstanceType: FunctionInstance> {
    poison_pill_receiver: tokio::sync::broadcast::Receiver<()>,
    function_instance: Option<Box<FunctionInstanceType>>,
    guest_api_host: Option<super::guest_api::GuestAPIHost>,
    telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
    guest_api_host_register: std::sync::Arc<tokio::sync::Mutex<Box<dyn super::runtime::GuestAPIHostRegister + Send>>>,
    code: Vec<u8>,
    data_plane: edgeless_dataplane::handle::DataplaneHandle,
    serialized_state: Option<String>,
    init_payload: Option<String>,
    runtime_api: futures::channel::mpsc::UnboundedSender<super::runtime::RuntimeRequest>,
    instance_id: edgeless_api::function_instance::InstanceId,
    tracer_provider: opentelemetry_sdk::trace::TracerProvider,
    tracing_context: std::sync::Arc<tokio::sync::Mutex<TracingContext>>,
}
pub struct TracingContext {
    pub tracer: opentelemetry_sdk::trace::Tracer,
    pub parent_context: opentelemetry::Context
}

impl<FunctionInstanceType: FunctionInstance> FunctionInstanceRunner<FunctionInstanceType> {
    pub async fn new(
        spawn_req: edgeless_api::function_instance::SpawnFunctionRequest,
        data_plane: edgeless_dataplane::handle::DataplaneHandle,
        runtime_api: futures::channel::mpsc::UnboundedSender<super::runtime::RuntimeRequest>,
        state_handle: Box<dyn crate::state_management::StateHandleAPI>,
        telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
        guest_api_host_register: std::sync::Arc<tokio::sync::Mutex<Box<dyn super::runtime::GuestAPIHostRegister + Send>>>
    ) -> Self {
        let instance_id = spawn_req.instance_id;
        let mut telemetry_handle = telemetry_handle;
        let mut state_handle = state_handle;
        let mut data_plane = data_plane;

        let (poison_pill_sender, poison_pill_receiver) = tokio::sync::broadcast::channel::<()>(1);
        let serialized_state = state_handle.get().await;

        let exporter = opentelemetry_otlp::SpanExporter::builder()
            .with_tonic()
            .with_endpoint("http://otelco:4317")
            .with_timeout(std::time::Duration::from_secs(3))
            .build().unwrap();
        
        let tracer_provider = opentelemetry_sdk::trace::TracerProvider::builder()
            .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
            .with_config(
                opentelemetry_sdk::trace::Config::default()
                .with_resource(opentelemetry_sdk::Resource::new(vec![
                    opentelemetry::KeyValue::new("service.name", spawn_req.code.function_class_id),
                    opentelemetry::KeyValue::new("component.instance_id", spawn_req.instance_id.function_id.to_string()),
                    opentelemetry::KeyValue::new("component.node_id", spawn_req.instance_id.node_id.to_string()),
                    opentelemetry::KeyValue::new("component.type", "actor"),
                    opentelemetry::KeyValue::new("actor.version", spawn_req.code.function_class_version.to_string()),
                    opentelemetry::KeyValue::new("actor.runtime", spawn_req.code.function_class_type.to_string())
                ]))
            )
            .build();

        let tracer = tracer_provider.tracer("actor_runtime");

        let tracing_context = std::sync::Arc::new(tokio::sync::Mutex::new(
            TracingContext {
                tracer: tracer.clone(),
                parent_context: opentelemetry::Context::new(),
            }
        ));

        data_plane.set_tracer(tracer);

        let guest_api_host = crate::base_runtime::guest_api::GuestAPIHost {
            instance_id,
            data_plane: data_plane.clone(),
            state_handle,
            telemetry_handle: telemetry_handle.fork(std::collections::BTreeMap::new()),
            poison_pill_receiver: poison_pill_sender.subscribe(),
            tracing_context: tracing_context.clone()
        };

        let task = Box::new(
            FunctionInstanceTask::<FunctionInstanceType>::new(
                poison_pill_receiver,
                telemetry_handle,
                guest_api_host_register,
                guest_api_host,
                spawn_req.code.function_class_code.clone(),
                data_plane.clone(),
                serialized_state,
                spawn_req.annotations.get("init-payload").cloned(),
                runtime_api,
                instance_id,
                tracer_provider,
                tracing_context
            )
            .await,
        );

        let task_handle = tokio::task::spawn(async move {
            let mut task = task;
            task.run().await;
        });

        Self {
            task_handle: Some(task_handle),
            poison_pill_sender,
            data_plane: data_plane.clone(),
            _instance: PhantomData {},
        }
    }

    pub async fn stop(&mut self) {
        self.poison_pill_sender.send(()).unwrap();

        if let Some(handle) = self.task_handle.take() {
            handle.await.unwrap();
        }
    }

    pub async fn patch(&mut self, update_request: edgeless_api::common::PatchRequest) {
        self.data_plane
            .update_mapping(update_request.input_mapping, update_request.output_mapping)
            .await;
    }
}

impl<FunctionInstanceType: FunctionInstance> FunctionInstanceTask<FunctionInstanceType> {
    pub async fn new(
        poison_pill_receiver: tokio::sync::broadcast::Receiver<()>,
        telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
        guest_api_host_register: std::sync::Arc<tokio::sync::Mutex<Box<dyn super::runtime::GuestAPIHostRegister + Send>>>,
        guest_api_host: super::guest_api::GuestAPIHost,
        code: Vec<u8>,
        data_plane: edgeless_dataplane::handle::DataplaneHandle,
        serialized_state: Option<String>,
        init_param: Option<String>,
        runtime_api: futures::channel::mpsc::UnboundedSender<super::runtime::RuntimeRequest>,
        instance_id: edgeless_api::function_instance::InstanceId,
        tracer_provider: opentelemetry_sdk::trace::TracerProvider,
        tracing_context: std::sync::Arc<tokio::sync::Mutex<TracingContext>>,
    ) -> Self {
        Self {
            poison_pill_receiver,
            function_instance: None,
            guest_api_host: Some(guest_api_host),
            telemetry_handle,
            guest_api_host_register,
            code,
            data_plane,
            serialized_state,
            init_payload: init_param,
            runtime_api,
            instance_id,
            tracer_provider,
            tracing_context,
        }
    }

    /// Function lifecycle; Runs until the poison pill is received or there is an error.
    /// Always calls the exit handler (with the exit status)
    pub async fn run(&mut self) {
        let mut res = self.instantiate().await;
        assert!(self.guest_api_host.is_none());
        if res.is_ok() {
            res = self.init().await;
        }
        if res.is_ok() {
            res = self.processing_loop().await;
        }
        self.guest_api_host_register.lock().await.deregister_guest_api_host(&self.instance_id);
        self.exit(res).await;
    }

    async fn instantiate(&mut self) -> Result<(), super::FunctionInstanceError> {
        // self.data_plane.set_tracer(self.tracing_context.lock().await.tracer.clone());

        let start = tokio::time::Instant::now();
        let mut span = self.tracing_context.lock().await.tracer.start("instantiate");

        let runtime_configuration;
        {
            // Register this function instance, if needed by the runtime.
            let mut register = self.guest_api_host_register.lock().await;
            if register.needs_to_register() {
                register.register_guest_api_host(&self.instance_id, self.guest_api_host.take().unwrap());
            }
            runtime_configuration = register.configuration();
        }

        self.function_instance =
            Some(FunctionInstanceType::instantiate(&self.instance_id, runtime_configuration, &mut self.guest_api_host.take(), &self.code).await?);

        span.end();

        self.telemetry_handle.observe(
            edgeless_telemetry::telemetry_events::TelemetryEvent::FunctionInstantiate(start.elapsed()),
            std::collections::BTreeMap::new(),
        );

        Ok(())
    }

    async fn init(&mut self) -> Result<(), super::FunctionInstanceError> {
        let start = tokio::time::Instant::now();
        let mut span = self.tracing_context.lock().await.tracer.start("init");

        self.function_instance
            .as_mut()
            .ok_or(super::FunctionInstanceError::InternalError)?
            .init(self.init_payload.as_deref(), self.serialized_state.as_deref())
            .await?;

        span.end();

        self.telemetry_handle.observe(
            edgeless_telemetry::telemetry_events::TelemetryEvent::FunctionInit(start.elapsed()),
            std::collections::BTreeMap::new(),
        );

        Ok(())
    }

    async fn processing_loop(&mut self) -> Result<(), super::FunctionInstanceError> {
        // let mut poison_pill_recv = Box::pin(self.poison_pill_receiver.recv()).fuse();
        loop {
            futures::select! {
                // Given each function instance is an independent task, the runtime needs to send a poison pill to cleanly stop it (processed here)
                _ = Box::pin(self.poison_pill_receiver.recv()).fuse() => {
                    return self.stop().await;
                },
                // Receive a normal event from the dataplane and invoke the function instance
                edgeless_dataplane::core::DataplaneEvent{source_id, channel_id, message, target_port, context: span_context} =  Box::pin(self.data_plane.receive_next()).fuse() => {
                    // let mut context = opentelemetry::Context::new();
                    // context = context.with_remote_span_context(span_context);
                    // context = context.with_value(opentelemetry::KeyValue::new("actor.id", self.instance_id.to_string()));
                    //  context);
                    self.process_message(
                        source_id,
                        channel_id,
                        message,
                        target_port,
                        span_context
                    ).await?;
                }
            }
        }
    }

    async fn process_message(
        &mut self,
        source_id: edgeless_api::function_instance::InstanceId,
        channel_id: u64,
        message: edgeless_dataplane::core::Message,
        target_port: edgeless_api::function_instance::PortId,
        context: opentelemetry::trace::SpanContext
    ) -> Result<(), super::FunctionInstanceError> {
        match message {
            edgeless_dataplane::core::Message::Cast(payload) => self.process_cast_message(source_id, target_port, payload, context).await,
            edgeless_dataplane::core::Message::Call(payload) => self.process_call_message(source_id, target_port, payload, channel_id, context).await,
            _ => {
                log::debug!("Unprocessed Message");
                Ok(())
            }
        }
    }

    async fn process_cast_message(
        &mut self,
        source_id: edgeless_api::function_instance::InstanceId,
        target_port: edgeless_api::function_instance::PortId,
        payload: String,
        span_context: opentelemetry::trace::SpanContext
    ) -> Result<(), super::FunctionInstanceError> {
        
        let start = tokio::time::Instant::now();
        let mut span = self.span(format!("process_cast_{}", target_port.0), span_context, Some(target_port.clone())).await;
        let context = opentelemetry::Context::with_span(&opentelemetry::Context::new(), span );
        self.tracing_context.lock().await.parent_context = context;

        self.function_instance
            .as_mut()
            .ok_or(super::FunctionInstanceError::InternalError)?
            .cast(&source_id, target_port.0.as_str(), &payload)
            .await?;

        // span.end();
        self.tracing_context.lock().await.parent_context = opentelemetry::Context::new();
        self.telemetry_handle.observe(
            edgeless_telemetry::telemetry_events::TelemetryEvent::FunctionInvocationCompleted(start.elapsed()),
            std::collections::BTreeMap::from([("EVENT_TYPE".to_string(), "CAST".to_string())]),
        );
        Ok(())
    }

    async fn process_call_message(
        &mut self,
        source_id: edgeless_api::function_instance::InstanceId,
        target_port: edgeless_api::function_instance::PortId,
        payload: String,
        channel_id: u64,
        span_context: opentelemetry::trace::SpanContext
    ) -> Result<(), super::FunctionInstanceError> {
        let start = tokio::time::Instant::now();

        let span = self.span(format!("process_call_{}", target_port.0), span_context, Some(target_port.clone())).await;
        self.tracing_context.lock().await.parent_context = opentelemetry::Context::with_span(&opentelemetry::Context::new(), span );

        let res = self
            .function_instance
            .as_mut()
            .ok_or(super::FunctionInstanceError::InternalError)?
            .call(&source_id, target_port.0.as_str(), &payload)
            .await?;

        self.tracing_context.lock().await.parent_context = opentelemetry::Context::new();
        self.telemetry_handle.observe(
            edgeless_telemetry::telemetry_events::TelemetryEvent::FunctionInvocationCompleted(start.elapsed()),
            std::collections::BTreeMap::from([("EVENT_TYPE".to_string(), "CALL".to_string())]),
        );

        let mut wh = self.data_plane.clone();
        wh.reply(source_id, channel_id, res).await;
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), super::FunctionInstanceError> {
        let start = tokio::time::Instant::now();
        let mut span = self.span("process_stop".to_string(), opentelemetry::trace::SpanContext::empty_context(), None).await;

        self.function_instance
            .as_mut()
            .ok_or(super::FunctionInstanceError::InternalError)?
            .stop()
            .await?;

        self.telemetry_handle.observe(
            edgeless_telemetry::telemetry_events::TelemetryEvent::FunctionStop(start.elapsed()),
            std::collections::BTreeMap::new(),
        );

        Ok(())
    }

    async fn exit(&mut self, exit_status: Result<(), super::FunctionInstanceError>) {
        self.runtime_api
            .send(super::runtime::RuntimeRequest::FunctionExit(self.instance_id, exit_status.clone()))
            .await
            .unwrap_or_else(|_| log::error!("FunctionInstance outlived runner."));

        self.telemetry_handle.observe(
            edgeless_telemetry::telemetry_events::TelemetryEvent::FunctionExit(match exit_status {
                Ok(_) => edgeless_telemetry::telemetry_events::FunctionExitStatus::Ok,
                Err(exit_err) => match exit_err {
                    FunctionInstanceError::BadCode => edgeless_telemetry::telemetry_events::FunctionExitStatus::CodeError,
                    _ => edgeless_telemetry::telemetry_events::FunctionExitStatus::InternalError,
                },
            }),
            std::collections::BTreeMap::new(),
        );
    }

    async fn span(
        &mut self,
        span_id: String,
        parent: opentelemetry::trace::SpanContext,
        input_port: Option<edgeless_api::function_instance::PortId>
    ) -> opentelemetry_sdk::trace::Span {
        let tracer = self.tracing_context.lock().await.tracer.clone();
        let context = opentelemetry::Context::current();
        let mut span = if parent.is_valid() {
            assert!(parent.is_sampled());
            let context = context.with_remote_span_context(parent);
            context.span().add_event("msg_received", Vec::new());
            context.span().end();
            tracer.start_with_context(span_id, &context)
        } else {
            assert!(false);
            tracer.start(span_id)
        };
        span.add_event("test", vec![]);
        if let Some(input_port) = &input_port {
            span.set_attribute(opentelemetry::KeyValue::new("component.input_port", input_port.0.clone()));
        }
        // span.set_attribute(opentelemetry::KeyValue::new("actor.id", "example2"));
        span
    }
}
