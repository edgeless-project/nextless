// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-FileCopyrightText: © 2023 University of Cambridge, System Research Group
// SPDX-FileCopyrightText: © 2024 Roman Kolcun <roman.kolcun@cl.cam.ac.uk>
// SPDX-License-Identifier: MIT

use crate::ir::interaction::dialect::DialectConstraint;

#[derive(Clone)]
pub struct WorkerNode {
    agent_url: String,
    invocation_url: String,
    api: Box<dyn edgeless_api::agent::AgentAPI>,
    resource_providers: std::collections::HashMap<String, super::resource_provider::ResourceProvider>,
    capabilities: edgeless_api::node_registration::NodeCapabilities,
    health_status: edgeless_api::node_management::HealthStatus,
    supported_link_types: std::collections::HashMap<edgeless_api::link::LinkType, edgeless_api::link::LinkProviderId>,
    // This should probably be based on link types and is a placeholder
    is_proxy: bool,
    #[allow(unused)]
    telemetry_provider: Option<Box<dyn crate::ir::TelemetryProvider>>,
    id: edgeless_api::function_instance::NodeId,
    cluster_id: edgeless_api::function_instance::NodeId,
}

#[derive(thiserror::Error, Debug)]
pub enum NodeError {
    #[error("API Request Failed")]
    RequestFailed,
    #[error("Node is Unreachable")]
    Unreachable,
}

pub type NodeResult = Result<(), NodeError>;

impl WorkerNode {
    #[allow(clippy::too_many_arguments)]
    pub async fn new(
        node_id: uuid::Uuid,
        agent_url: String,
        invocation_url: String,
        resource_providers: Vec<edgeless_api::node_registration::ResourceProviderSpecification>,
        capabilities: edgeless_api::node_registration::NodeCapabilities,
        link_providers: Vec<edgeless_api::node_registration::LinkProviderSpecification>,
        telemetry_provider: Option<Box<dyn crate::ir::TelemetryProvider>>,
        cluster_id: edgeless_api::function_instance::NodeId,
    ) -> Self {
        let api = get_api_for_url(&agent_url).await;

        Self {
            agent_url,
            invocation_url: invocation_url.clone(),
            api,
            resource_providers: resource_providers
                .into_iter()
                .map(|r| {
                    (
                        r.provider_id,
                        super::resource_provider::ResourceProvider {
                            class_type: r.class_type,
                            outputs: r.outputs,
                        },
                    )
                })
                .collect(),
            capabilities,
            health_status: edgeless_api::node_management::HealthStatus::empty(),
            supported_link_types: link_providers.into_iter().map(|p| (p.class, p.provider_id)).collect(),
            is_proxy: true,
            id: node_id,
            cluster_id,
            telemetry_provider,
        }
    }

    pub fn agent_url(&self) -> String {
        self.agent_url.clone()
    }

    pub fn invocation_url(&self) -> String {
        self.invocation_url.clone()
    }

    pub fn fn_client(
        &mut self,
    ) -> Option<Box<dyn edgeless_api::function_instance::FunctionInstanceAPI<edgeless_api::function_instance::InstanceId>>> {
        Some(self.api.function_instance_api())
    }

    pub fn resource_client(
        &mut self,
    ) -> Option<Box<dyn edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::InstanceId>>> {
        Some(self.api.resource_configuration_api())
    }

    pub fn proxy_client(&mut self) -> Option<Box<dyn edgeless_api::proxy_instance::ProxyInstanceAPI>> {
        Some(self.api.proxy_instance_api())
    }

    pub fn link_instance_client(&mut self) -> Option<Box<dyn edgeless_api::link::LinkInstanceAPI>> {
        Some(self.api.link_instance_api())
    }

    pub async fn update_peers(&mut self, updates: &[edgeless_api::node_management::UpdatePeersRequest]) -> NodeResult {
        let filtered_updates = updates.iter().filter(|u| match u {
            edgeless_api::node_management::UpdatePeersRequest::Add(id, _) => id != &self.id,
            edgeless_api::node_management::UpdatePeersRequest::Del(id) => id != &self.id,
            edgeless_api::node_management::UpdatePeersRequest::Clear => true,
        });

        for update in filtered_updates {
            self.api
                .node_management_api()
                .update_peers(update.clone())
                .await
                .map_err(|_| NodeError::RequestFailed)?;
        }

        Ok(())
    }

    pub async fn health_check(&mut self) -> NodeResult {
        match self.api.node_management_api().keep_alive().await {
            Ok(health_status) => {
                self.health_status = health_status;
                Ok(())
            }
            Err(_) => Err(NodeError::Unreachable),
        }
    }
}

impl crate::ir::Node for WorkerNode {
    fn available_runtimes<'a>(&'a self) -> std::collections::HashMap<String, crate::ir::Runtime<'a>> {
        self.capabilities
            .runtimes
            .iter()
            .filter_map(|rt| match rt.base_type.as_str() {
                "WASM" => {
                    let features = rt
                        .features
                        .iter()
                        .filter_map(|feature| match feature.as_str() {
                            "WGPU" => Some(crate::ir::behavior::dialect::wasm::WasmDialectFeatures::Wgpu),
                            _ => {
                                tracing::warn!("Node announced unknown feature");
                                None
                            }
                        })
                        .collect();

                    Some(("WASM".to_string(), crate::ir::Runtime::WasmBase(self, features)))
                }
                "NATIVE_DYNAMIC" => {
                    let mut features: std::collections::BTreeSet<crate::ir::behavior::dialect::native_dyanamic::NativeDynamicDialectFeatures> = rt
                        .features
                        .iter()
                        .filter_map(|feature| match feature.as_str() {
                            "AES" => Some(crate::ir::behavior::dialect::native_dyanamic::NativeDynamicDialectFeatures::Aes),
                            _ => {
                                tracing::warn!("Node announced unknown feature");
                                None
                            }
                        })
                        .collect();

                    features.insert(match self.capabilities.cpu_arch.as_str() {
                        "x86_64" => crate::ir::behavior::dialect::native_dyanamic::NativeDynamicDialectFeatures::Amd64,
                        "aarch64" => crate::ir::behavior::dialect::native_dyanamic::NativeDynamicDialectFeatures::Aarch64,
                        _ => {
                            tracing::warn!("Unsupported Arch");
                            return None;
                        }
                    });
                    Some(("NATIVE_DYNAMIC".to_string(), crate::ir::Runtime::NativeBase(self, features)))
                }
                _ => None,
            })
            .collect()
    }

    fn available_resource_providers<'a>(&'a self) -> crate::ir::ResourceProviders<'a> {
        self.resource_providers
            .iter()
            .map(|(k, v)| (k.clone(), v as &dyn crate::ir::ResourceProvider))
            .collect()
    }

    fn available_link_types(&self) -> crate::ir::LinkProviders {
        self.supported_link_types.clone()
    }

    fn available_interaction_dialects(&self) -> Vec<crate::ir::interaction::dialect::DialectDescriptor> {
        let mut dialects = Vec::new();

        dialects.push(crate::ir::interaction::dialect::DialectDescriptor {
            base_type: crate::ir::interaction::dialect::physical_overlay::ID,
            constraints: std::collections::BTreeSet::from([crate::ir::interaction::dialect::physical_overlay::PhysicalOverlayConstraint::Cluster(
                self.cluster_id.clone(),
            )
            .as_container()]),
        });

        if self
            .supported_link_types
            .contains_key(&edgeless_api::link::LinkType("MULTICAST".to_string()))
        {
            dialects.push(crate::ir::interaction::dialect::DialectDescriptor {
                base_type: crate::ir::interaction::dialect::ip_multicast::ID,
                constraints: std::collections::BTreeSet::from([crate::ir::interaction::dialect::ip_multicast::IpMulticastConstraint::Cluster(
                    self.cluster_id.clone(),
                )
                .as_container()]),
            });
        }

        dialects
    }

    fn labels(&self) -> Vec<String> {
        self.capabilities.labels.clone()
    }

    fn is_proxy(&self) -> bool {
        self.is_proxy
    }

    fn node_id(&self) -> edgeless_api::function_instance::NodeId {
        self.id
    }

    fn cluster_id(&self) -> edgeless_api::function_instance::NodeId {
        self.cluster_id
    }
}

impl crate::ir::WasmRuntime for WorkerNode {
    fn num_cores(&self) -> u32 {
        self.capabilities.num_cores
    }

    fn cpu_freq_hz(&self) -> f32 {
        self.capabilities.clock_freq_cpu
    }

    fn mem_size_bytes(&self) -> u32 {
        self.capabilities.mem_size
    }

    fn runtime_info(&self) -> Option<Box<dyn crate::ir::WasmRuntimeInfo>> {
        self.telemetry_provider.as_ref().map(|t| t.wasm_runtime_statistics_for(&self.id))
    }
}

impl crate::ir::NativeRuntime for WorkerNode {
    fn num_cores(&self) -> u32 {
        self.capabilities.num_cores
    }

    fn cpu_freq_hz(&self) -> f32 {
        self.capabilities.clock_freq_cpu
    }

    fn mem_size_bytes(&self) -> u32 {
        self.capabilities.mem_size
    }

    fn runtime_info(&self) -> Option<Box<dyn crate::ir::WasmRuntimeInfo>> {
        todo!()
    }
}

async fn get_api_for_url(agent_url: &str) -> Box<dyn edgeless_api::agent::AgentAPI + Send> {
    let (proto, host, port) = edgeless_api::util::parse_http_host(agent_url).unwrap();
    match proto {
        edgeless_api::util::Proto::COAP => {
            let addr = std::net::SocketAddrV4::new(host.parse().unwrap(), port);
            Box::new(edgeless_api::coap_impl::CoapClient::new(addr).await)
        }
        _ => Box::new(edgeless_api::grpc_impl::agent::AgentAPIClient::new(agent_url).await),
    }
}
