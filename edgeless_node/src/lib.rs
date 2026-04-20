// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

pub mod agent;
pub mod base_runtime;
pub mod proxy;
pub mod resources;
pub mod state_management;
#[cfg(any(feature = "wasmtime", test))]
pub mod wasm_runner;
#[cfg(any(feature = "wasmi", test))]
pub mod wasmi_runner;

pub mod native_runner;

pub mod dataplane;

pub mod telemetry;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EdgelessNodeSettings {
    /// General settings.
    pub general: EdgelessNodeGeneralSettings,
    /// Wasmtime run-time settings. Disabled if not present.
    pub wasmtime_runtime: Option<EdgelessNodeWasmtimeRuntimeSettings>,
    /// Container run-time settings.  Disabled if not present.
    pub wasmi_runtime: Option<EdgelessNodeWasmiRuntimeSettings>,
    /// Native runtime settings. Disabled if not present.
    pub native_runtime: Option<NativeRuntimeSettings>,
    /// Resource settings.
    pub resources: Option<EdgelessNodeResourceSettings>,
    /// User-specific capabilities.
    pub user_node_capabilities: Option<NodeCapabilitiesUser>,
    /// OpenTelemetry Export Settings
    pub opentelemetry_export: Option<OpenTelemetryExportConfig>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EdgelessNodeWasmtimeRuntimeSettings {
    /// True if WASM is enabled.
    pub enabled: bool,
    pub wgpu: Option<bool>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EdgelessNodeWasmiRuntimeSettings {
    /// True if WASM is enabled.
    pub enabled: bool,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct NativeRuntimeSettings {
    /// True if the native runtime is enabled.
    pub enabled: bool,
    pub aes: Option<bool>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct OpenTelemetryExportConfig {
    pub enabled: bool,
    pub endpoint: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EdgelessNodeGeneralSettings {
    /// The UUID of this node.
    pub node_id: uuid::Uuid,
    /// The URL of the agent of this node used for creating the local server.
    pub agent_url: String,
    /// The agent URL announced by the node.
    /// It is the end-point used by the controller to manage the node.
    /// It can be different from `agent_url`, e.g., for NAT traversal.
    pub agent_url_announced: String,
    /// The URL of the dataplane of this node, used for event dispatching.
    pub invocation_url: String,
    /// The invocation URL announced by the node.
    /// It can be different from `agent_url`, e.g., for NAT traversal.
    pub invocation_url_announced: String,
    /// The COAP URL of the dataplane of this node, used for event dispatching.
    pub invocation_url_coap: Option<String>,
    /// The COAP invocation URL announced by the node.
    /// It can be different from `agent_url`, e.g., for NAT traversal.
    pub invocation_url_announced_coap: Option<String>,
    /// The URL exposed by this node to publish telemetry metrics collected.
    pub metrics_url: String,
    /// The URL of the controller to which this node registers.
    pub controller_url: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EdgelessNodeResourceSettings {
    /// If `http_ingress_provider` is not empty, this is the URL of the
    /// HTTP web server exposed by the http-ingress resource for this node.
    pub http_ingress_url: Option<String>,
    /// If not empty, a http-ingress resource provider with that name is created.
    pub http_ingress_provider: Option<String>,
    /// If not empty, a http-egress resource provider with that name is created.
    pub http_egress_provider: Option<String>,
    /// If not empty, a file-log resource provider with that name is created.
    /// The resource will write on the local filesystem.
    pub file_log_provider: Option<String>,
    /// If not empty, a redis resource provider with that name is created.
    /// The resource will connect to a remote Redis server to update the
    /// value of a given given, as specified in the resource configuration
    /// at run-time.
    pub redis_provider: Option<String>,
    /// Support for RGB LED Matrix
    pub led_matrix: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct NodeCapabilitiesUser {
    pub num_cpus: Option<u32>,
    pub model_name_cpu: Option<String>,
    pub clock_freq_cpu: Option<f32>,
    pub num_cores: Option<u32>,
    pub cpu_arch: Option<String>,
    pub mem_size: Option<u32>,
    pub labels: Option<Vec<String>>,
    pub is_tee_running: Option<bool>,
    pub has_tpm: Option<bool>,
}

impl NodeCapabilitiesUser {
    pub fn empty() -> Self {
        Self {
            num_cpus: None,
            model_name_cpu: None,
            clock_freq_cpu: None,
            num_cores: None,
            cpu_arch: None,
            mem_size: None,
            labels: None,
            is_tee_running: None,
            has_tpm: None,
        }
    }
}

impl EdgelessNodeSettings {
    /// Create settings for a node with WASM run-time and no resources
    /// binding the given ports on the same address.
    pub fn new_without_resources(controller_url: &str, node_address: &str, agent_port: u16, invocation_port: u16, metrics_port: u16) -> Self {
        let agent_url = format!("http://{node_address}:{agent_port}");
        let invocation_url = format!("http://{node_address}:{invocation_port}");
        let invocation_url_coap = Some(format!("coap://{node_address}:{invocation_port}"));
        Self {
            general: EdgelessNodeGeneralSettings {
                node_id: uuid::Uuid::new_v4(),
                agent_url: agent_url.clone(),
                agent_url_announced: agent_url,
                invocation_url: invocation_url.clone(),
                invocation_url_announced: invocation_url,
                invocation_url_coap: invocation_url_coap.clone(),
                invocation_url_announced_coap: invocation_url_coap,
                metrics_url: format!("http://{node_address}:{metrics_port}"),
                controller_url: controller_url.to_string(),
            },
            wasmtime_runtime: Some(EdgelessNodeWasmtimeRuntimeSettings { enabled: true, wgpu: None }),
            wasmi_runtime: None,
            native_runtime: None,
            resources: None,
            user_node_capabilities: None,
            opentelemetry_export: None,
        }
    }
}

fn get_capabilities(
    runtimes: Vec<edgeless_api::node_registration::RuntimeType>,
    user_node_capabilities: NodeCapabilitiesUser,
) -> edgeless_api::node_registration::NodeCapabilities {
    let mut sys = sysinfo::System::new_all();
    sys.refresh_all();
    sys.refresh_cpu_all();
    std::thread::sleep(std::time::Duration::from_millis(250));
    sys.refresh_cpu_all();
    let mut model_name_set = std::collections::HashSet::new();
    let mut clock_freq_cpu_set = std::collections::HashSet::new();
    for processor in sys.cpus() {
        model_name_set.insert(processor.brand());
        clock_freq_cpu_set.insert(processor.frequency());
    }
    let model_name_cpu = match model_name_set.iter().next() {
        Some(val) => val.to_string(),
        None => "".to_string(),
    };
    if model_name_set.len() > 1 {
        tracing::warn!("CPUs have different models, using: {model_name_cpu}");
    }
    let clock_freq_cpu = match clock_freq_cpu_set.iter().next() {
        Some(val) => *val as f32,
        None => 0.0,
    };
    if clock_freq_cpu_set.len() > 1 {
        tracing::warn!("CPUs have different frequencies, using: {clock_freq_cpu}");
    }

    edgeless_api::node_registration::NodeCapabilities {
        num_cpus: user_node_capabilities.num_cpus.unwrap_or(sys.cpus().len() as u32),
        model_name_cpu: user_node_capabilities.model_name_cpu.unwrap_or(model_name_cpu),
        clock_freq_cpu: user_node_capabilities.clock_freq_cpu.unwrap_or(clock_freq_cpu),
        num_cores: user_node_capabilities
            .num_cores
            .unwrap_or(sysinfo::System::physical_core_count().unwrap_or(1) as u32),
        cpu_arch: sysinfo::System::cpu_arch(),
        sys: if sysinfo::System::kernel_long_version().to_lowercase().contains("darwin") {
            "darwin".to_string()
        } else {
            "linux".to_string()
        },
        mem_size: user_node_capabilities.mem_size.unwrap_or((sys.total_memory() / 1024 / 1024) as u32),
        labels: user_node_capabilities.labels.unwrap_or_default(),
        is_tee_running: user_node_capabilities.is_tee_running.unwrap_or(false),
        has_tpm: user_node_capabilities.has_tpm.unwrap_or(false),
        runtimes,
    }
}

async fn fill_resources(
    data_plane: crate::dataplane::handle::DataplaneProvider,
    node_id: uuid::Uuid,
    settings: &Option<EdgelessNodeResourceSettings>,
    provider_specifications: &mut Vec<edgeless_api::node_registration::ResourceProviderSpecification>,
    simulator_display_sender: std::sync::mpsc::Sender<edgeless_function_types::led_matrix::MatrixFrame>,
) -> std::collections::HashMap<String, agent::ResourceDesc> {
    let mut ret = std::collections::HashMap::<String, agent::ResourceDesc>::new();

    if let Some(settings) = settings {
        if let (Some(http_ingress_url), Some(provider_id)) = (&settings.http_ingress_url, &settings.http_ingress_provider) {
            if !http_ingress_url.is_empty() && !provider_id.is_empty() {
                let class_type = "http-ingress".to_string();
                tracing::info!("Creating resource '{provider_id}' at {http_ingress_url}");
                ret.insert(
                    provider_id.clone(),
                    agent::ResourceDesc {
                        class_type: class_type.clone(),
                        instance_limit: None,
                        client: resources::http_ingress::ingress_task(
                            data_plane.clone(),
                            edgeless_api::function_instance::InstanceId::new(node_id),
                            http_ingress_url.clone(),
                        )
                        .await,
                    },
                );
                provider_specifications.push(edgeless_api::node_registration::ResourceProviderSpecification {
                    provider_id: provider_id.clone(),
                    class_type,
                    outputs: vec!["new_request".to_string()],
                    instance_limit: None,
                });
            }
        }

        if let Some(provider_id) = &settings.http_egress_provider {
            if !provider_id.is_empty() {
                tracing::info!("Creating resource '{provider_id}'");
                let class_type = "http-egress".to_string();
                ret.insert(
                    provider_id.clone(),
                    agent::ResourceDesc {
                        class_type: class_type.clone(),
                        instance_limit: None,
                        client: Box::new(
                            resources::http_egress::EgressResourceProvider::new(
                                data_plane.clone(),
                                edgeless_api::function_instance::InstanceId::new(node_id),
                            )
                            .await,
                        ),
                    },
                );
                provider_specifications.push(edgeless_api::node_registration::ResourceProviderSpecification {
                    provider_id: provider_id.clone(),
                    class_type,
                    outputs: vec![],
                    instance_limit: None,
                });
            }
        }

        if let Some(provider_id) = &settings.file_log_provider {
            if !provider_id.is_empty() {
                tracing::info!("Creating resource '{provider_id}'");
                let class_type = "file-log".to_string();
                ret.insert(
                    provider_id.clone(),
                    agent::ResourceDesc {
                        class_type: class_type.clone(),
                        instance_limit: None,
                        client: Box::new(
                            resources::file_log::FileLogResourceProvider::new(
                                data_plane.clone(),
                                edgeless_api::function_instance::InstanceId::new(node_id),
                            )
                            .await,
                        ),
                    },
                );
                provider_specifications.push(edgeless_api::node_registration::ResourceProviderSpecification {
                    provider_id: provider_id.clone(),
                    class_type,
                    outputs: vec![],
                    instance_limit: None,
                });
            }
        }

        if let Some(provider_id) = &settings.redis_provider {
            if !provider_id.is_empty() {
                tracing::info!("Creating resource '{provider_id}'");
                let class_type = "redis".to_string();
                ret.insert(
                    provider_id.clone(),
                    agent::ResourceDesc {
                        class_type: class_type.clone(),
                        instance_limit: None,
                        client: Box::new(
                            resources::redis::RedisResourceProvider::new(
                                data_plane.clone(),
                                edgeless_api::function_instance::InstanceId::new(node_id),
                            )
                            .await,
                        ),
                    },
                );
                provider_specifications.push(edgeless_api::node_registration::ResourceProviderSpecification {
                    provider_id: provider_id.clone(),
                    class_type,
                    outputs: vec![],
                    instance_limit: None,
                });
            }
        }

        if let Some(provider_id) = &settings.led_matrix {
            if !provider_id.is_empty() {
                tracing::info!("Creating resource '{provider_id}'");
                let class_type = "led-matrix".to_string();
                ret.insert(
                    provider_id.clone(),
                    agent::ResourceDesc {
                        class_type: class_type.clone(),
                        instance_limit: Some(1),
                        client: Box::new(
                            resources::led_matrix::LedMatrixResourceProvider::new(
                                data_plane.clone(),
                                edgeless_api::function_instance::InstanceId::new(node_id),
                                simulator_display_sender,
                            )
                            .await,
                        ),
                    },
                );
                provider_specifications.push(edgeless_api::node_registration::ResourceProviderSpecification {
                    provider_id: provider_id.clone(),
                    class_type,
                    outputs: vec![],
                    instance_limit: Some(1),
                });
            }
        }
    }
    ret
}

pub async fn edgeless_node_main(
    settings: EdgelessNodeSettings,
    simulator_display_sender: std::sync::mpsc::Sender<edgeless_function_types::led_matrix::MatrixFrame>,
) {
    tracing::info!("Starting Edgeless Node");
    tracing::debug!("Settings: {settings:?}");

    let mut async_tasks: Vec<tokio::task::JoinHandle<()>> = Vec::new();

    // Create the state manager.
    let state_manager = Box::new(state_management::StateManager::new().await);

    // Create the data plane.
    let data_plane = crate::dataplane::handle::DataplaneProvider::new(
        settings.general.node_id,
        settings.general.invocation_url.clone(),
        settings.general.invocation_url_coap.clone(),
    )
    .await;

    // Create the telemetry provider.
    let telemetry_provider = crate::telemetry::telemetry_events::TelemetryProcessor::new(settings.general.metrics_url.clone())
        .await
        .unwrap_or_else(|_| panic!("could not build the telemetry provider at URL {}", &settings.general.metrics_url));

    // List of runners supported by this node to be filled below depending on
    // the node's configuration.
    let mut runners =
        std::collections::HashMap::<edgeless_api::node_registration::RuntimeType, Box<dyn crate::base_runtime::RuntimeAPI + Send>>::new();

    // Create the wasmtime runtime, if needed.
    #[cfg(feature = "wasmtime")]
    if let Some(wasmtime_runtime_settings) = settings.wasmtime_runtime {
        if wasmtime_runtime_settings.enabled {
            let (wasmtime_runtime_client, mut wasmtime_runtime_task_s) = base_runtime::runtime::create::<
                wasm_runner::function_instance::WASMFunctionInstance,
                base_runtime::function_instance_runner::FunctionInstanceRunner<wasm_runner::function_instance::WASMFunctionInstance>,
            >(
                data_plane.clone(),
                state_manager.clone(),
                Box::new(telemetry_provider.get_handle(std::collections::BTreeMap::from([
                    ("FUNCTION_TYPE".to_string(), "WASM".to_string()),
                    ("WASM_RUNTIME".to_string(), "wasmtime".to_string()),
                    ("NODE_ID".to_string(), settings.general.node_id.to_string()),
                ]))),
            );

            let mut features = Vec::new();
            if wasmtime_runtime_settings.wgpu.unwrap_or(false) {
                features.push("WGPU".to_string());
            }

            let runtime_type = edgeless_api::node_registration::RuntimeType {
                base_type: "WASM".to_string(),
                features,
            };

            runners.insert(runtime_type, Box::new(wasmtime_runtime_client.clone()));
            async_tasks.push(tokio::spawn(async move {
                wasmtime_runtime_task_s.run().await;
            }));
        }
    };

    // Create the wasmi (wasm_base) runtime, if needed.
    #[cfg(feature = "wasmi")]
    if let Some(wasm_runtime_settings) = settings.wasmi_runtime {
        if wasm_runtime_settings.enabled {
            let (wasmi_runtime_client, mut wasmi_runtime_task_s) = base_runtime::runtime::create::<
                wasm_runner::function_instance::WASMFunctionInstance,
                base_runtime::function_instance_runner::FunctionInstanceRunner<wasm_runner::function_instance::WASMFunctionInstance>,
            >(
                data_plane.clone(),
                state_manager.clone(),
                Box::new(telemetry_provider.get_handle(std::collections::BTreeMap::from([
                    ("FUNCTION_TYPE".to_string(), "WASM".to_string()),
                    ("WASM_RUNTIME".to_string(), "wasmi".to_string()),
                    ("NODE_ID".to_string(), settings.general.node_id.to_string()),
                ]))),
            );

            let runtime_type = edgeless_api::node_registration::RuntimeType {
                base_type: "WASM".to_string(),
                features: Vec::new(),
            };

            runners.insert(runtime_type, Box::new(wasmi_runtime_client.clone()));
            async_tasks.push(tokio::spawn(async move {
                wasmi_runtime_task_s.run().await;
            }));
        }
    }

    if let Some(native_runtime_settings) = settings.native_runtime {
        if native_runtime_settings.enabled {
            let (native_runtime_client, mut native_runtime_task) = base_runtime::runtime::create::<
                native_runner::NativeFunctionInstance,
                base_runtime::function_instance_runner_thread::FunctionInstanceRunner<native_runner::NativeFunctionInstance>,
            >(
                data_plane.clone(),
                state_manager.clone(),
                Box::new(telemetry_provider.get_handle(std::collections::BTreeMap::from([
                    ("FUNCTION_TYPE".to_string(), "NATIVE_DYNAMIC".to_string()),
                    ("NODE_ID".to_string(), settings.general.node_id.to_string()),
                ]))),
            );

            let mut features = Vec::new();
            if native_runtime_settings.aes.unwrap_or(false) {
                features.push("AES".to_string());
            }

            let runtime_type = edgeless_api::node_registration::RuntimeType {
                base_type: "NATIVE_DYNAMIC".to_string(),
                features,
            };

            runners.insert(runtime_type, Box::new(native_runtime_client.clone()));
            async_tasks.push(tokio::spawn(async move {
                native_runtime_task.run().await;
            }));
        }
    };

    // Create the resources.
    let mut resource_provider_specifications = vec![];
    let resources = fill_resources(
        data_plane.clone(),
        settings.general.node_id,
        &settings.resources,
        &mut resource_provider_specifications,
        simulator_display_sender,
    )
    .await;

    let proxy_manager = Box::new(proxy::ProxyManager::start(data_plane.clone()).await);

    // Create the agent.
    let runtimes = runners.keys().cloned().collect::<Vec<_>>();
    let (mut agent, agent_task) = agent::Agent::new(
        runners,
        resources,
        settings.general.node_id,
        data_plane.clone(),
        proxy_manager,
        settings.general.controller_url.clone(),
        agent::NodeUrls {
            agent_url: if !settings.general.agent_url_announced.is_empty() {
                settings.general.agent_url_announced.clone()
            } else {
                settings.general.agent_url.clone()
            },
            invocation_url_grpc: if !settings.general.invocation_url_announced.is_empty() {
                Some(settings.general.invocation_url_announced.clone())
            } else if !settings.general.invocation_url.is_empty() {
                Some(settings.general.invocation_url.clone())
            } else {
                None
            },
            invocation_url_coap: settings
                .general
                .invocation_url_announced_coap
                .clone()
                .or(settings.general.invocation_url_coap.clone()),
        },
        get_capabilities(runtimes, settings.user_node_capabilities.unwrap_or(NodeCapabilitiesUser::empty())),
    );
    async_tasks.push(tokio::task::spawn(agent_task));

    let cloned_agent_url = settings.general.agent_url.clone();
    async_tasks.push(tokio::task::spawn(async move {
        edgeless_api::grpc_impl::agent::AgentAPIServer::run(agent.get_api_client(), cloned_agent_url).await
    }));

    if let Ok((_, ip_str, _)) = edgeless_api::util::parse_http_host(&settings.general.agent_url) {
        if let Ok(ip) = ip_str.parse::<std::net::IpAddr>() {
            if !ip.is_loopback() {
                tracing::warn!("Agent gRPC server listening on IP {ip_str}. As nextless does not currently contain any security features, it should only receive traffic from fully trusted networks!")
            }
        }
    }

    // Wait for all the tasks to complete.
    let _ = futures::future::join_all(async_tasks).await;
}

pub fn edgeless_node_default_conf() -> String {
    let caps = get_capabilities(
        vec![edgeless_api::node_registration::RuntimeType {
            base_type: "WASM".to_string(),
            features: Vec::new(),
        }],
        NodeCapabilitiesUser::empty(),
    );

    format!(
        "{}num_cpus = {}\nmodel_name_cpu = \"{}\"\nclock_freq_cpu = {}\nnum_cores = {}\nmem_size = {}\n{}",
        r##"[general]
node_id = "fda6ce79-46df-4f96-a0d2-456f720f606c"
agent_url = "http://127.0.0.1:7021"
agent_url_announced = ""
invocation_url = "http://127.0.0.1:7002"
invocation_url_announced = ""
invocation_url_coap = "coap://127.0.0.1:7002"
invocation_url_announced_coap = ""
metrics_url = "http://127.0.0.1:7003"
controller_url = "http://127.0.0.1:7001"

[wasm_runtime]
enabled = true

[container_runtime]
enabled = false
guest_api_host_url = "http://127.0.0.1:7100"

[resources]
http_ingress_url = "http://127.0.0.1:7035"
http_ingress_provider = "http-ingress-1"
http_egress_provider = "http-egress-1"
file_log_provider = "file-log-1"
redis_provider = "redis-1"

[user_node_capabilities]
"##,
        caps.num_cpus,
        caps.model_name_cpu,
        caps.clock_freq_cpu,
        caps.num_cores,
        caps.mem_size,
        r##"labels = []
is_tee_running = false
has_tpm = false"##
    )
}
