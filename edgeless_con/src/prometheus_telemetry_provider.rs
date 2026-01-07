use std::str::FromStr;

#[derive(Clone)]
pub struct PrometheusTelemetryProvider {
    url: String,
}

impl PrometheusTelemetryProvider {
    pub fn new(url: String) -> Self {
        Self { url }
    }
}

impl crate::ir::TelemetryProvider for PrometheusTelemetryProvider {
    fn component_statistics_for(&self, component_id: &edgeless_api::function_instance::InstanceId) -> Box<dyn crate::ir::ComponentRuntimeStatistics> {
        Box::new(PrometheusComponentRuntimeStatistics::new(*component_id, &self.url))
    }

    fn input_port_statistics_for(
        &self,
        component_id: &edgeless_api::function_instance::InstanceId,
        port_id: &edgeless_api::function_instance::PortId,
    ) -> Box<dyn crate::ir::PortStatistics> {
        Box::new(PrometheusPortStatistics::new(
            *component_id,
            port_id.clone(),
            PortDirection::Input,
            &self.url,
        ))
    }

    fn output_port_statistics_for(
        &self,
        component_id: &edgeless_api::function_instance::InstanceId,
        port_id: &edgeless_api::function_instance::PortId,
    ) -> Box<dyn crate::ir::PortStatistics> {
        Box::new(PrometheusPortStatistics::new(
            *component_id,
            port_id.clone(),
            PortDirection::Output,
            &self.url,
        ))
    }

    fn wasm_runtime_statistics_for(&self, node_id: &edgeless_api::function_instance::NodeId) -> Box<dyn crate::ir::WasmRuntimeInfo> {
        Box::new(PrometheusWasmRuntimeInfo::new(node_id, &self.url))
    }
}

struct PrometheusComponentRuntimeStatistics {
    component_id: edgeless_api::function_instance::InstanceId,
    client: prometheus_http_query::Client,
}

struct PrometheusPortStatistics {
    component_id: edgeless_api::function_instance::InstanceId,
    port_id: edgeless_api::function_instance::PortId,
    client: prometheus_http_query::Client,
    direction: PortDirection,
}

struct PrometheusWasmRuntimeInfo {
    #[allow(unused)]
    node_id: edgeless_api::function_instance::NodeId,
    #[allow(unused)]
    client: prometheus_http_query::Client,
}

enum PortDirection {
    Input,
    Output,
}

impl PrometheusComponentRuntimeStatistics {
    fn new(component_id: edgeless_api::function_instance::InstanceId, url: &str) -> Self {
        PrometheusComponentRuntimeStatistics {
            component_id,
            client: prometheus_http_query::Client::from_str(url).expect("Critical Prometheus Configuration Error"),
        }
    }
}

impl crate::ir::ComponentRuntimeStatistics for PrometheusComponentRuntimeStatistics {
    fn invocation_rate_abs(&self, period: std::time::Duration) -> Option<f64> {
        let query = format!(
            "sum(rate(execution_time_seconds_count{{node_id = \"{}\", function_id = \"{}\"}}[{}]))",
            self.component_id.node_id,
            self.component_id.function_id,
            format_args!("{}s", period.as_secs())
        );

        single_value_helper(&self.client, query)
    }

    fn invocations_rate_abs_by_port(&self, period: std::time::Duration) -> Vec<(edgeless_api::function_instance::PortId, f64)> {
        let query = format!(
            "rate(execution_time_seconds_count{{node_id = \"{}\", function_id = \"{}\"}}[{}])",
            self.component_id.node_id,
            self.component_id.function_id,
            format_args!("{}s", period.as_secs())
        );
        let res_f = self.client.query(query).post();
        let res = tokio::runtime::Handle::current().block_on(res_f);
        if let Ok(res) = res {
            if let Ok(val_array) = res.into_inner().0.into_vector() {
                return val_array
                    .iter()
                    .filter_map(|i| {
                        Some((
                            edgeless_api::function_instance::PortId(i.metric().get("port")?.clone()),
                            i.sample().value(),
                        ))
                    })
                    .collect();
            }
        }
        vec![]
    }

    fn duration_mean_secs(&self, period: std::time::Duration) -> Option<f64> {
        let selector_time = format!(
            "{{node_id = \"{}\", function_id = \"{}\"}}[{}]",
            self.component_id.node_id,
            self.component_id.function_id,
            format_args!("{}s", period.as_secs())
        );

        let query = format!("sum(increase(execution_time_seconds_sum{selector_time}))/sum(increase(execution_time_seconds_count{selector_time}))");

        single_value_helper(&self.client, query)
    }

    fn duration_soft_limit_rate_rel(&self, period: std::time::Duration) -> Option<f64> {
        let selector_time = format!(
            "{{node_id = \"{}\", function_id = \"{}\"}}[{}]",
            self.component_id.node_id,
            self.component_id.function_id,
            format_args!("{}s", period.as_secs())
        );

        let query =
            format!("(sum(increase(under_soft_limit_total{selector_time})) or vector(0))/sum(increase(execution_time_seconds_count{selector_time}))");

        single_value_helper(&self.client, query)
    }

    fn error_rate_rel(&self, period: std::time::Duration) -> Option<f64> {
        let selector_time = format!(
            "{{node_id = \"{}\", function_id = \"{}\"}}[{}]",
            self.component_id.node_id,
            self.component_id.function_id,
            format_args!("{}s", period.as_secs())
        );

        let query = format!("(sum(increase(errors_total{selector_time})) or vector(0))/sum(increase(execution_time_seconds_count{selector_time}))");

        single_value_helper(&self.client, query)
    }
}

impl PrometheusPortStatistics {
    fn new(
        component_id: edgeless_api::function_instance::InstanceId,
        port_id: edgeless_api::function_instance::PortId,
        direction: PortDirection,
        url: &str,
    ) -> Self {
        PrometheusPortStatistics {
            client: prometheus_http_query::Client::from_str(url).expect("Critical Prometheus Configuration Error"),
            component_id,
            port_id,
            direction,
        }
    }

    fn selector_and_period(&self, period: std::time::Duration) -> String {
        match self.direction {
            PortDirection::Input => self.input_selector_and_period(period),
            PortDirection::Output => self.output_selector_and_period(period),
        }
    }

    fn input_selector_and_period(&self, period: std::time::Duration) -> String {
        format!(
            "{{dest_node_id=\"{}\", dest_function_id=\"{}\", dest_port=\"{}\"}}[{}]",
            self.component_id.node_id,
            self.component_id.function_id,
            self.port_id.0,
            format_args!("{}s", period.as_secs())
        )
    }

    fn output_selector_and_period(&self, period: std::time::Duration) -> String {
        format!(
            "{{source_node_id=\"{}\", source_function_id=\"{}\", source_port=\"{}\"}}[{}]",
            self.component_id.node_id,
            self.component_id.function_id,
            self.port_id.0,
            format_args!("{}s", period.as_secs())
        )
    }
}

impl crate::ir::PortStatistics for PrometheusPortStatistics {
    fn message_rate_abs(&self, period: std::time::Duration) -> Option<f64> {
        let query = format!("sum(rate(message_size_bytes_count{}))", self.selector_and_period(period));
        single_value_helper(&self.client, query)
    }

    fn message_rate_abs_by_peer(&self, period: std::time::Duration) -> Vec<(edgeless_api::function_instance::InstanceId, f64)> {
        let query = format!("rate(message_size_bytes_count{})", self.selector_and_period(period));
        let res_f = self.client.query(query).post();
        let res = tokio::runtime::Handle::current().block_on(res_f);
        if let Ok(res) = res {
            if let Ok(val_array) = res.into_inner().0.into_vector() {
                return val_array
                    .iter()
                    .filter_map(|i| {
                        Some((
                            if let PortDirection::Input = self.direction {
                                edgeless_api::function_instance::InstanceId {
                                    node_id: uuid::Uuid::parse_str(i.metric().get("source_node_id")?).ok()?,
                                    function_id: uuid::Uuid::parse_str(i.metric().get("source_function_id")?).ok()?,
                                }
                            } else {
                                edgeless_api::function_instance::InstanceId {
                                    node_id: uuid::Uuid::parse_str(i.metric().get("dest_node_id")?).ok()?,
                                    function_id: uuid::Uuid::parse_str(i.metric().get("dest_function_id")?).ok()?,
                                }
                            },
                            i.sample().value(),
                        ))
                    })
                    .collect();
            }
        }
        vec![]
    }

    fn message_size_mean_bytes(&self, period: std::time::Duration) -> Option<f64> {
        let selector_and_period = self.selector_and_period(period);

        let query =
            format!("sum(increase(message_size_bytes_sum{selector_and_period}))/sum(increase(message_size_bytes_count{selector_and_period}))");

        single_value_helper(&self.client, query)
    }

    fn message_size_mean_byte_by_peer(&self, period: std::time::Duration) -> Vec<(edgeless_api::function_instance::InstanceId, f64)> {
        let selector_and_period = self.selector_and_period(period);

        let query = format!("increase(message_size_bytes_sum{selector_and_period})/increase(message_size_bytes_count{selector_and_period})");
        let res_f = self.client.query(query).post();
        let res = tokio::runtime::Handle::current().block_on(res_f);
        if let Ok(res) = res {
            if let Ok(val_array) = res.into_inner().0.into_vector() {
                return val_array
                    .iter()
                    .filter_map(|i| {
                        Some((
                            edgeless_api::function_instance::InstanceId {
                                node_id: uuid::Uuid::parse_str(i.metric().get("source_node_id")?).ok()?,
                                function_id: uuid::Uuid::parse_str(i.metric().get("source_function_id")?).ok()?,
                            },
                            i.sample().value(),
                        ))
                    })
                    .collect();
            }
        }
        vec![]
    }
}

impl PrometheusWasmRuntimeInfo {
    fn new(node_id: &edgeless_api::function_instance::NodeId, url: &str) -> Self {
        Self {
            node_id: *node_id,
            client: prometheus_http_query::Client::from_str(url).expect("Critical Prometheus Configuration Error"),
        }
    }
}

impl crate::ir::WasmRuntimeInfo for PrometheusWasmRuntimeInfo {
    fn cpu_load(&self) -> f32 {
        0.0
    }

    fn mem_used(&self) -> f32 {
        0.0
    }

    fn running_instances(&self) -> u32 {
        0
    }
}

fn single_value_helper(client: &prometheus_http_query::Client, query: String) -> Option<f64> {
    let res_f = client.query(query.clone()).post();
    let res = tokio::runtime::Handle::current().block_on(res_f);
    if let Ok(res) = res {
        if let Ok(val) = res.into_inner().0.into_vector() {
            return Some(val.first()?.sample().value());
        }
    }
    tracing::warn!("Prometheus Query Failed: {query}");
    None
}
