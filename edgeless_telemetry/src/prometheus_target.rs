// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT
use warp::Filter;

/// Prometheus collects metrics from targets by scraping metrics HTTP targets. This struct defines that.
pub struct PrometheusEventTarget {
    _registry: std::sync::Arc<tokio::sync::Mutex<prometheus_client::registry::Registry>>,
    execution_times: prometheus_client::metrics::family::Family<ExecutionLabels, prometheus_client::metrics::histogram::Histogram>,
    errors: prometheus_client::metrics::family::Family<ExecutionLabels, prometheus_client::metrics::counter::Counter>,
    under_soft_limit: prometheus_client::metrics::family::Family<ExecutionLabels, prometheus_client::metrics::counter::Counter>,
    message_sizes: prometheus_client::metrics::family::Family<MessageLabels, prometheus_client::metrics::histogram::Histogram>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, prometheus_client::encoding::EncodeLabelSet)]
struct RuntimeLabels {
    node_id: String,
    function_type: String,
}

// TODO: add additional labels like class_spec, function_name
#[derive(Clone, Debug, Hash, PartialEq, Eq, prometheus_client::encoding::EncodeLabelSet)]
struct FunctionLabels {
    node_id: String,
    function_id: String,
    function_type: String,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, prometheus_client::encoding::EncodeLabelValue)]
enum InvocationType {
    Cast,
    Call,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, prometheus_client::encoding::EncodeLabelSet)]
struct ExecutionLabels {
    node_id: String,
    function_id: String,
    function_type: String,
    invocation_type: InvocationType,
    port: String,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, prometheus_client::encoding::EncodeLabelSet)]
struct MessageLabels {
    dest_node_id: String,
    dest_function_id: String,
    source_function_id: String,
    source_node_id: String,
    dest_port: String,
    source_port: String,
}

impl PrometheusEventTarget {
    pub async fn new(endpoint: &str) -> Self {
        let registry = std::sync::Arc::new(tokio::sync::Mutex::new(<prometheus_client::registry::Registry>::default()));

        let under_soft_limit = prometheus_client::metrics::family::Family::<ExecutionLabels, prometheus_client::metrics::counter::Counter>::default();

        let errors = prometheus_client::metrics::family::Family::<ExecutionLabels, prometheus_client::metrics::counter::Counter>::default();

        let execution_time_seconds =
            prometheus_client::metrics::family::Family::<ExecutionLabels, prometheus_client::metrics::histogram::Histogram>::new_with_constructor(
                || {
                    let buckets = [0.00001, 0.0001, 0.001, 0.01, 0.1, 1.0, 10.0];
                    prometheus_client::metrics::histogram::Histogram::new(buckets.into_iter())
                },
            );

        let message_size_bytes =
            prometheus_client::metrics::family::Family::<MessageLabels, prometheus_client::metrics::histogram::Histogram>::new_with_constructor(
                || {
                    let buckets = [50.0, 250.0, 500.0, 750.0, 1000.0, 1250.0, 1500.0, 10000.0];
                    prometheus_client::metrics::histogram::Histogram::new(buckets.into_iter())
                },
            );

        // let error_count

        registry.lock().await.register("under_soft_limit", "", under_soft_limit.clone());
        registry.lock().await.register("message_size_bytes", "", message_size_bytes.clone());
        registry
            .lock()
            .await
            .register("execution_time_seconds", "", execution_time_seconds.clone());

        let reg_clone = registry.clone();
        let socket_addr: std::net::SocketAddr = endpoint.parse().unwrap_or_else(|_| panic!("invalid endpoint: {}", &endpoint));
        tokio::spawn(async move {
            let metric_handler = warp::path("metrics").then(move || {
                let cloned = reg_clone.clone();
                async move {
                    let mut buffer = String::new();
                    match prometheus_client::encoding::text::encode(&mut buffer, &*cloned.lock().await) {
                        Ok(_) => warp::http::Response::builder()
                            .header("Content-Type", "application/openmetrics-text; version=1.0.0; charset=utf-8")
                            .body(buffer),
                        Err(_) => warp::http::Response::builder().status(500).body("".to_string()),
                    }
                }
            });

            warp::serve(metric_handler).run(socket_addr).await;
        });

        Self {
            _registry: registry,
            execution_times: execution_time_seconds,
            under_soft_limit,
            errors,
            message_sizes: message_size_bytes,
        }
    }
}

impl crate::telemetry_events::EventProcessor for PrometheusEventTarget {
    fn handle(
        &mut self,
        event: &crate::telemetry_events::TelemetryEvent,
        event_tags: &std::collections::BTreeMap<String, String>,
    ) -> crate::telemetry_events::TelemetryProcessingResult {
        match event {
            crate::telemetry_events::TelemetryEvent::MessageReceived(size) => {
                if let (Some(node_id), Some(function_id), Some(source_node_id), Some(source_function_id), Some(source_port), Some(dest_port)) = (
                    event_tags.get("NODE_ID"),
                    event_tags.get("FUNCTION_ID"),
                    event_tags.get("SOURCE_NODE_ID"),
                    event_tags.get("SOURCE_FUNCTION_ID"),
                    event_tags.get("SOURCE_PORT"),
                    event_tags.get("DEST_PORT"),
                ) {
                    self.message_sizes
                        .get_or_create(&MessageLabels {
                            dest_node_id: node_id.clone(),
                            dest_function_id: function_id.clone(),
                            source_function_id: source_function_id.clone(),
                            source_node_id: source_node_id.clone(),
                            dest_port: dest_port.clone(),
                            source_port: source_port.clone(),
                        })
                        .observe(*size as f64)
                }
            }
            crate::telemetry_events::TelemetryEvent::FunctionInvocationCompleted {
                duration,
                error,
                under_duration_soft_limit,
            } => {
                if let (Some(node_id), Some(function_id), Some(function_type), Some(invoction_type), Some(port)) = (
                    event_tags.get("NODE_ID"),
                    event_tags.get("FUNCTION_ID"),
                    event_tags.get("FUNCTION_TYPE"),
                    event_tags.get("EVENT_TYPE"),
                    event_tags.get("PORT"),
                ) {
                    let labels = ExecutionLabels {
                        node_id: node_id.to_string(),
                        function_type: function_type.to_string(),
                        function_id: function_id.to_string(),
                        invocation_type: match invoction_type.as_str() {
                            "CALL" => InvocationType::Call,
                            _ => InvocationType::Cast,
                        },
                        port: port.to_string(),
                    };
                    self.execution_times.get_or_create(&labels).observe(duration.as_secs_f64());
                    if *under_duration_soft_limit {
                        self.under_soft_limit.get_or_create(&labels).inc();
                    } else {
                    }
                    if *error {
                        self.errors.get_or_create(&labels).inc();
                    }
                }
            }
            _ => {
                return crate::telemetry_events::TelemetryProcessingResult::PASSED;
            }
        }
        crate::telemetry_events::TelemetryProcessingResult::FINAL
    }
}
