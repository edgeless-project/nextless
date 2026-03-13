// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT
#[derive(Debug, PartialEq, Eq)]
pub enum TelemetryLogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl std::fmt::Display for TelemetryLogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TelemetryLogLevel::Error => f.write_str("Error"),
            TelemetryLogLevel::Warn => f.write_str("Warn"),
            TelemetryLogLevel::Info => f.write_str("Info"),
            TelemetryLogLevel::Debug => f.write_str("Debug"),
            TelemetryLogLevel::Trace => f.write_str("Trace"),
        }
    }
}

pub fn api_to_telemetry(lvl: String) -> TelemetryLogLevel {
    match lvl.as_str() {
        "Trace" => TelemetryLogLevel::Trace,
        "Debug" => TelemetryLogLevel::Debug,
        "Info" => TelemetryLogLevel::Info,
        "Warn" => TelemetryLogLevel::Warn,
        _ => TelemetryLogLevel::Error,
    }
}

pub fn telemetry_to_api(lvl: TelemetryLogLevel) -> String {
    match lvl {
        TelemetryLogLevel::Trace => "Trace".to_string(),
        TelemetryLogLevel::Debug => "Debug".to_string(),
        TelemetryLogLevel::Info => "Info".to_string(),
        TelemetryLogLevel::Warn => "Warn".to_string(),
        TelemetryLogLevel::Error => "Error".to_string(),
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum FunctionExitStatus {
    Ok,
    InternalError,
    CodeError,
}

#[derive(Debug, PartialEq, Eq)]
pub enum TelemetryEvent {
    FunctionInstantiate(std::time::Duration),
    FunctionInit(std::time::Duration),
    FunctionLogEntry(TelemetryLogLevel, String, String), // (_, target, msg)
    FunctionInvocationCompleted {
        duration: std::time::Duration,
        error: bool,
        under_duration_soft_limit: bool,
    },
    FunctionStop(std::time::Duration),
    FunctionExit(FunctionExitStatus),
    MessageReceived(u64),
}

#[derive(Clone)]
pub struct TelemetryHandle {
    handle_tags: std::collections::BTreeMap<String, String>,
    sender: tokio::sync::mpsc::UnboundedSender<TelemetryProcessorInput>,
}

pub trait TelemetryHandleAPI: TelemetryHandleAPIClone + Sync + Send {
    fn observe(&mut self, event: TelemetryEvent, event_tags: std::collections::BTreeMap<String, String>);
    fn fork(&mut self, child_tags: std::collections::BTreeMap<String, String>) -> Box<dyn TelemetryHandleAPI>;
}

// https://stackoverflow.com/a/30353928
pub trait TelemetryHandleAPIClone {
    fn clone_box(&self) -> Box<dyn TelemetryHandleAPI>;
}
impl<T> TelemetryHandleAPIClone for T
where
    T: 'static + TelemetryHandleAPI + Clone,
{
    fn clone_box(&self) -> Box<dyn TelemetryHandleAPI> {
        Box::new(self.clone())
    }
}
impl Clone for Box<dyn TelemetryHandleAPI> {
    fn clone(&self) -> Box<dyn TelemetryHandleAPI> {
        self.clone_box()
    }
}

impl TelemetryHandleAPI for TelemetryHandle {
    fn observe(&mut self, event: TelemetryEvent, event_tags: std::collections::BTreeMap<String, String>) {
        let mut event_tags = event_tags;
        let mut merged_tags = self.handle_tags.clone();
        merged_tags.append(&mut event_tags);

        if self.sender.send(TelemetryProcessorInput::TelemetryEvent(event, merged_tags)).is_err() {
            tracing::error!("Tried to observe telemetry while the receiver is stopped.")
        }
    }

    fn fork(&mut self, child_tags: std::collections::BTreeMap<String, String>) -> Box<dyn TelemetryHandleAPI> {
        let mut child_tags = child_tags;
        let mut merged_tags = self.handle_tags.clone();
        merged_tags.append(&mut child_tags);
        Box::new(TelemetryHandle {
            handle_tags: merged_tags,
            sender: self.sender.clone(),
        })
    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum TelemetryProcessingResult {
    PASSED,
    PROCESSED,
    FINAL,
}

#[derive(Debug)]
enum TelemetryProcessorInput {
    TelemetryEvent(TelemetryEvent, std::collections::BTreeMap<String, String>),
}

pub trait EventProcessor: Sync + Send {
    fn handle(&mut self, event: &TelemetryEvent, event_tags: &std::collections::BTreeMap<String, String>) -> TelemetryProcessingResult;
}

// https://stackoverflow.com/a/33206814
static COLORS: [&str; 12] = ["31", "32", "33", "34", "35", "36", "91", "92", "93", "94", "95", "96"];

#[derive(Default)]
struct EventLogger {
    next_color: usize,
    colors: std::collections::HashMap<String, &'static str>,
    output_configuration: OutputConfiguration,
}

#[derive(Debug)]
struct OutputConfiguration {
    instantiate: bool,
    init: bool,
    log_entry: bool,
    invocation_completed: bool,
    stop: bool,
    exit: bool,
    message_reception: bool,
}

impl Default for OutputConfiguration {
    fn default() -> Self {
        Self {
            instantiate: true,
            init: true,
            log_entry: true,
            invocation_completed: false,
            stop: true,
            exit: true,
            message_reception: false,
        }
    }
}

impl OutputConfiguration {
    fn new_from_env() -> Self {
        let mut instance = Self::default();

        if let Ok(config_str) = std::env::var("EDGELESS_TELEMETRY_LOG") {
            for item in config_str.split(",") {
                if let Some((key, value)) = item.split_once("=") {
                    let value: bool = value.parse().unwrap_or(false);
                    match key {
                        "instantiate" => instance.instantiate = value,
                        "init" => instance.init = value,
                        "log_entry" => instance.log_entry = value,
                        "invocation_completed" => instance.invocation_completed = value,
                        "stop" => instance.stop = value,
                        "exit" => instance.exit = value,
                        "message_reception" => instance.message_reception = value,
                        _ => {}
                    }
                }
            }
        }

        instance
    }
}

impl EventLogger {
    fn new() -> Self {
        Self {
            output_configuration: OutputConfiguration::new_from_env(),
            ..Default::default()
        }
    }
}

impl EventProcessor for EventLogger {
    fn handle(&mut self, event: &TelemetryEvent, event_tags: &std::collections::BTreeMap<String, String>) -> TelemetryProcessingResult {
        let f_id = event_tags.get("FUNCTION_ID").unwrap();

        // Escape needs special escale sequence https://github.com/rust-lang/rust/issues/30491
        let color = self.colors.entry(f_id.clone()).or_insert_with(|| {
            let c = COLORS[self.next_color];
            self.next_color += 1;
            if self.next_color == COLORS.len() {
                self.next_color = 0;
            }
            c
        });

        match event {
            TelemetryEvent::FunctionLogEntry(level, component, message) => {
                if self.output_configuration.log_entry {
                    println!(
                        "\x1b[{}m{}\x1b[0m:{}",
                        color,
                        f_id,
                        format_args!("[{}][{}] {}", level, component, message)
                    );
                }
            }
            TelemetryEvent::MessageReceived(message_len) => {
                if self.output_configuration.message_reception {
                    println!("\x1b[{color}m{f_id}\x1b[0m: Message Received; Size: {message_len}");
                }
            }
            TelemetryEvent::FunctionInvocationCompleted {
                duration,
                error,
                under_duration_soft_limit,
            } => {
                if self.output_configuration.invocation_completed {
                    println!("\x1b[{color}m{f_id}\x1b[0m: Invocation Completed; Duration: {duration:?}; Error: {error}; Under Soft Limit: {under_duration_soft_limit}");
                }
            }
            TelemetryEvent::FunctionInstantiate(duration) => {
                if self.output_configuration.instantiate {
                    println!("\x1b[{color}m{f_id}\x1b[0m: Actor Instantiated; Duration: {duration:?}");
                }
            }
            TelemetryEvent::FunctionInit(duration) => {
                if self.output_configuration.init {
                    println!("\x1b[{color}m{f_id}\x1b[0m: Actor Initialized; Duration: {duration:?}");
                }
            }
            TelemetryEvent::FunctionStop(duration) => {
                if self.output_configuration.stop {
                    println!("\x1b[{color}m{f_id}\x1b[0m: Actor Stopped; Duration: {duration:?}");
                }
            }
            TelemetryEvent::FunctionExit(function_exit_status) => {
                if self.output_configuration.exit {
                    println!("\x1b[{color}m{f_id}\x1b[0m: Actor Instantiated; ExitStatus: {function_exit_status:?}");
                }
            }
        }

        TelemetryProcessingResult::PROCESSED
    }
}

struct TelemetryProcessorInner {
    processing_chain: Vec<Box<dyn EventProcessor>>,
    receiver: tokio::sync::mpsc::UnboundedReceiver<TelemetryProcessorInput>,
}

impl TelemetryProcessorInner {
    async fn run(&mut self) {
        while let Some(val) = self.receiver.recv().await {
            match val {
                TelemetryProcessorInput::TelemetryEvent(event, event_tags) => {
                    self.handle(event, event_tags).await;
                }
            }
        }
    }

    async fn handle(&mut self, event: TelemetryEvent, event_tags: std::collections::BTreeMap<String, String>) {
        for processor in &mut self.processing_chain {
            let processing_result = processor.handle(&event, &event_tags);
            if processing_result == TelemetryProcessingResult::FINAL {
                break;
            }
        }
    }
}

pub struct TelemetryProcessor {
    sender: tokio::sync::mpsc::UnboundedSender<TelemetryProcessorInput>,
}

impl TelemetryProcessor {
    pub async fn new(metrics_url: String) -> anyhow::Result<Self> {
        match edgeless_api::util::parse_http_host(&metrics_url) {
            Ok((_, ip, port)) => {
                let (sender, receiver) = tokio::sync::mpsc::unbounded_channel::<TelemetryProcessorInput>();

                let inner = TelemetryProcessorInner {
                    processing_chain: vec![
                        Box::new(super::file_logger::FileLogger::new()),
                        Box::new(EventLogger::new()),
                        Box::new(crate::prometheus_target::PrometheusEventTarget::new(&format!("{}:{}", &ip, port)).await),
                    ],
                    receiver,
                };

                tokio::spawn(async move {
                    let mut inner = inner;
                    inner.run().await;
                });

                Ok(Self { sender })
            }
            Err(err) => Err(err),
        }
    }

    pub fn get_handle(&self, handle_tags: std::collections::BTreeMap<String, String>) -> TelemetryHandle {
        TelemetryHandle {
            handle_tags,
            sender: self.sender.clone(),
        }
    }
}
