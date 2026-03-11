use std::time::Duration;

// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
use edgeless_api::{controller::ControllerAPI, proxy_instance::ProxyInstanceAPI};
use edgeless_dataplane::core::EdgelessDataplanePeerSettings;
use futures::{Future, SinkExt, StreamExt};

enum AgentRequest {
    Spawn(Box<edgeless_api::function_instance::SpawnFunctionRequest>),
    SpawnResource(
        edgeless_api::resource_configuration::ResourceInstanceSpecification,
        futures::channel::oneshot::Sender<anyhow::Result<edgeless_api::common::StartComponentResponse<edgeless_api::function_instance::InstanceId>>>,
    ),
    Stop(edgeless_api::function_instance::InstanceId),
    StopResource(
        edgeless_api::function_instance::InstanceId,
        futures::channel::oneshot::Sender<anyhow::Result<()>>,
    ),
    Patch(edgeless_api::common::PatchRequest),
    PatchResource(edgeless_api::common::PatchRequest, futures::channel::oneshot::Sender<anyhow::Result<()>>),
    UpdatePeers(edgeless_api::node_management::UpdatePeersRequest),
    HealthStatus(futures::channel::oneshot::Sender<anyhow::Result<edgeless_api::node_management::HealthStatus>>),
    CreateLink(edgeless_api::link::CreateLinkRequest),
    RemoveLink(edgeless_api::link::LinkInstanceId),
    StartProxy(edgeless_api::proxy_instance::ProxySpec),
    PatchProxy(edgeless_api::proxy_instance::ProxySpec),
    StopProxy(edgeless_api::function_instance::InstanceId),
}

pub struct Agent {
    sender: futures::channel::mpsc::UnboundedSender<AgentRequest>,
}

pub struct NodeUrls {
    pub agent_url: String,
    pub invocation_url_grpc: Option<String>,
    pub invocation_url_coap: Option<String>,
}

pub struct AgentTask {
    node_id: uuid::Uuid,
    actor_runtimes: std::collections::HashMap<edgeless_api::node_registration::RuntimeType, Box<dyn crate::base_runtime::RuntimeAPI + Send>>,
    resource_providers: std::collections::HashMap<String, ResourceDesc>,
    proxy: Box<dyn ProxyInstanceAPI>,
    dataplane_provider: edgeless_dataplane::handle::DataplaneProvider,

    // After spawning a new function, the function´s class is only used to determine which runner to start it on.
    // When stopping, only the stop_function_id is provided which does not allow to know which runner it is
    // currently deployed on. Here, we implement a instance_id -> function_class HashMap
    actor_instance_runtime_map: std::collections::HashMap<edgeless_api::function_instance::InstanceId, String>,
    resource_instance_provider_map: std::collections::HashMap<edgeless_api::function_instance::InstanceId, String>,

    sysinfo: sysinfo::System,
    main_pid: sysinfo::Pid,

    last_keepalive_timestamp: Option<tokio::time::Instant>,
    controller_url: String,
    capabilities: edgeless_api::node_registration::NodeCapabilities,
    node_urls: NodeUrls,
}

pub struct ResourceDesc {
    pub class_type: String,
    pub instance_limit: Option<u32>,
    pub client: Box<dyn edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::InstanceId>>,
}

impl Agent {
    pub fn new(
        runners: std::collections::HashMap<edgeless_api::node_registration::RuntimeType, Box<dyn crate::base_runtime::RuntimeAPI + Send>>,
        resources: std::collections::HashMap<String, ResourceDesc>,
        node_id: uuid::Uuid,
        data_plane_provider: edgeless_dataplane::handle::DataplaneProvider,
        proxy: Box<dyn ProxyInstanceAPI>,
        controller_url: String,
        node_urls: NodeUrls,
        capabilities: edgeless_api::node_registration::NodeCapabilities,
    ) -> (Self, std::pin::Pin<Box<dyn Future<Output = ()> + Send>>) {
        let (sender, receiver) = futures::channel::mpsc::unbounded();

        for class_type in runners.keys() {
            tracing::info!("Runner registered, class_type: {class_type}");
        }

        let mut agent_task = AgentTask {
            node_id,
            actor_runtimes: runners,
            resource_providers: resources,
            proxy: proxy,
            dataplane_provider: data_plane_provider,
            actor_instance_runtime_map: std::collections::HashMap::new(),
            resource_instance_provider_map: std::collections::HashMap::new(),
            sysinfo: sysinfo::System::new(),
            main_pid: sysinfo::Pid::from_u32(std::process::id()),
            last_keepalive_timestamp: None,
            controller_url,
            capabilities,
            node_urls,
        };

        let main_task = Box::pin(async move {
            agent_task.main_task(receiver).await;
        });

        (Agent { sender }, main_task)
    }

    pub fn get_api_client(&mut self) -> Box<dyn edgeless_api::agent::AgentAPI + Send> {
        Box::new(AgentClient {
            function_instance_client: Box::new(FunctionInstanceNodeClient { sender: self.sender.clone() }),
            node_management_client: Box::new(NodeManagementClient { sender: self.sender.clone() }),
            resource_configuration_client: Box::new(ResourceConfigurationClient { sender: self.sender.clone() }),
            link_instance_client: Box::new(LinkInstanceAPIClient { sender: self.sender.clone() }),
            proxy_instance_client: Box::new(ProxyInstanceAPIClient { sender: self.sender.clone() }),
        })
    }
}

impl AgentTask {
    async fn main_task(&mut self, receiver: futures::channel::mpsc::UnboundedReceiver<AgentRequest>) {
        let mut receiver = std::pin::pin!(receiver);

        tracing::info!("Starting Edgeless Agent");

        {
            let initial_delay = tokio::time::Duration::from_millis(100);
            tracing::info!("Delay initial registration with Controller by {initial_delay:?}");
            tokio::time::sleep(initial_delay).await;
            tracing::info!("Initial registration with Controller");
            self.register_with_controller().await;
        }

        let elapsed_since_last_contact = self
            .last_keepalive_timestamp
            .map(|last_contact| last_contact.elapsed())
            .unwrap_or_default();

        let keepalive_timeout_delay = std::cmp::max(Duration::from_secs(0), Duration::from_secs(10) - elapsed_since_last_contact);

        loop {
            match tokio::time::timeout(keepalive_timeout_delay, receiver.next()).await {
                Ok(message) => {
                    if let Some(req) = message {
                        self.handle_agent_request(req).await;
                    } else {
                        tracing::info!("Agent Exit");
                        return;
                    }
                }
                Err(_timeout) => {
                    self.handle_controller_loss().await;
                }
            }
        }
    }

    async fn handle_agent_request(&mut self, req: AgentRequest) {
        match req {
            AgentRequest::Spawn(spawn_req) => self.start_actor(spawn_req).await,
            AgentRequest::Stop(stop_function_id) => self.stop_actor(stop_function_id).await,
            // PatchRequest contains function_id: ComponentId
            AgentRequest::Patch(update) => self.patch_actor(update).await,
            AgentRequest::UpdatePeers(request) => self.update_peers(request).await,
            AgentRequest::SpawnResource(instance_specification, responder) => {
                let reply = self.start_resource(instance_specification).await;
                responder.send(reply).unwrap_or_else(|_| tracing::warn!("Responder Send Error"))
            }
            AgentRequest::StopResource(resource_id, responder) => {
                let reply = self.stop_resource(resource_id).await;
                responder.send(reply).unwrap_or_else(|_| tracing::warn!("Responder Send Error"))
            }
            AgentRequest::PatchResource(update, responder) => {
                let reply = self.patch_resource(update).await;
                responder.send(reply).unwrap_or_else(|_| tracing::warn!("Responder Send Error"))
            }
            AgentRequest::HealthStatus(responder) => {
                let reply = self.healt_status().await;
                responder.send(reply).unwrap_or_else(|_| tracing::warn!("Responder Send Error"))
            }
            AgentRequest::CreateLink(req) => {
                edgeless_api::link::LinkInstanceAPI::create(&mut self.dataplane_provider, req)
                    .await
                    .unwrap_or_else(|_| tracing::warn!("Unreported error while creating a link"));
            }
            AgentRequest::RemoveLink(id) => {
                edgeless_api::link::LinkInstanceAPI::remove(&mut self.dataplane_provider, id)
                    .await
                    .unwrap_or_else(|_| tracing::warn!("Unreported error while removing a link"));
            }
            AgentRequest::StartProxy(proxy_spec) => {
                self.proxy
                    .start(proxy_spec)
                    .await
                    .unwrap_or_else(|_| tracing::warn!("Unreported error while starting proxy"));
            }
            AgentRequest::PatchProxy(proxy_spec) => {
                self.proxy
                    .patch(proxy_spec)
                    .await
                    .unwrap_or_else(|_| tracing::warn!("Unreported error while patching proxy"));
            }
            AgentRequest::StopProxy(instance_id) => {
                self.proxy
                    .stop(instance_id)
                    .await
                    .unwrap_or_else(|_| tracing::warn!("Unreported error while stopping proxy"));
            }
        }
    }

    async fn handle_controller_loss(&mut self) {
        tracing::warn!("Connection to controller lost!");
        self.last_keepalive_timestamp = None;

        tracing::info!("Stop orphan actors");
        for (actor_id, runtime_id) in &self.actor_instance_runtime_map {
            if let Some((_, rt)) = self.actor_runtimes.iter_mut().find(|(k, _)| &k.base_type == runtime_id) {
                if let Err(e) = rt.stop(actor_id.clone()).await {
                    tracing::warn!("Stop Orphan Actor Error: {e:?}");
                }
            }
        }
        self.actor_instance_runtime_map.clear();

        tracing::info!("Stop orphan resources");
        for (resource_id, provider_id) in &self.resource_instance_provider_map {
            if let Some(provider) = self.resource_providers.get_mut(provider_id) {
                if let Err(e) = provider.client.stop(resource_id.clone()).await {
                    tracing::warn!("Stop Orphan Resource Error: {e:?}");
                }
            }
        }
        self.actor_instance_runtime_map.clear();

        self.register_with_controller().await;
    }

    async fn register_with_controller(&mut self) {
        tracing::info!(
            "Registering this node '{}' on controller with url '{}'. Capabilities: {}",
            &self.node_id,
            &self.controller_url,
            self.capabilities
        );

        let mut controller_api_client = edgeless_api::grpc_impl::controller::ControllerAPIClient::new(&self.controller_url)
            .await
            .node_registration_api();

        let registration_request = edgeless_api::node_registration::UpdateNodeRequest::Registration(
            self.node_id,
            self.node_urls.agent_url.clone(),
            self.node_urls
                .invocation_url_coap
                .clone()
                .or(self.node_urls.invocation_url_grpc.clone())
                .unwrap(),
            self.resource_providers
                .iter()
                .map(|(provider_id, resource)| edgeless_api::node_registration::ResourceProviderSpecification {
                    provider_id: provider_id.clone(),
                    class_type: resource.class_type.clone(),
                    // TODO(raphaelhetzel) Fix (and add inputs) or remove
                    outputs: vec![],
                    instance_limit: resource.instance_limit,
                })
                .collect(),
            self.capabilities.clone(),
            self.dataplane_provider
                .link_providers()
                .await
                .into_iter()
                .map(|(class, id)| edgeless_api::node_registration::LinkProviderSpecification { provider_id: id, class })
                .collect(),
        );

        match controller_api_client.update_node(registration_request).await {
            Ok(res) => match res {
                edgeless_api::node_registration::UpdateNodeResponse::ResponseError(err) => {
                    // TODO add retry logic
                    panic!("Could not register to e-ORC {}: {}", &self.controller_url, err)
                }
                edgeless_api::node_registration::UpdateNodeResponse::Accepted => {
                    tracing::info!(
                        "This node '{}' registered to controller with url '{}'",
                        &self.node_id,
                        &self.controller_url
                    )
                }
            },
            // TODO add retry logic
            Err(err) => panic!(
                "Channel error when registering to controller with url '{}': {}",
                &self.controller_url, err
            ),
        }
    }

    async fn start_actor(&mut self, spawn_req: Box<edgeless_api::function_instance::SpawnFunctionRequest>) {
        let code_size = spawn_req.code.function_class_code.len();
        let actor_class = spawn_req.code.function_class_id.clone();
        let actor_id = spawn_req.instance_id.function_id;
        let runner = spawn_req.code.function_class_type.clone();
        tracing::info!("Actor spawn: ID: {actor_id}, Size: {code_size}. Class: {actor_class}, Runner: {runner}");

        // We can assume that the Optional<instance_id> is present.
        if spawn_req.instance_id.is_none() {
            tracing::error!("No instance_id provided for SpawnFunctionRequest!");
            return;
        }
        self.actor_instance_runtime_map
            .insert(spawn_req.instance_id, spawn_req.code.function_class_type.clone());

        // Get runner for function_class of spawn_req
        match self
            .actor_runtimes
            .iter_mut()
            .find(|(k, _)| k.base_type == spawn_req.code.function_class_type)
        {
            Some((_, r)) => {
                // Forward the start request to the correct runner
                match r.start(*spawn_req).await {
                    Ok(_) => {}
                    Err(err) => {
                        tracing::error!("Unhandled start error: {err}");
                        return;
                    }
                }
            }
            None => {
                tracing::warn!("Could not find runner for {}", spawn_req.code.function_class_type);
                return;
            }
        }
    }

    async fn stop_actor(&mut self, stop_function_id: edgeless_api::function_instance::InstanceId) {
        tracing::debug!("Agent stop {stop_function_id:?}");

        // Get function class by looking it up in the instanceId->functionClass map
        let function_class: String = match self.actor_instance_runtime_map.get(&stop_function_id) {
            Some(v) => v.clone(),
            None => {
                tracing::error!("Could not identify runtime hosting actor: {stop_function_id}.");
                return;
            }
        };

        // Get runner for function_class
        match self.actor_runtimes.iter_mut().find(|(k, _)| k.base_type == function_class) {
            Some((_, r)) => {
                // Forward the stop request to the correct runner
                match r.stop(stop_function_id).await {
                    Ok(_) => {
                        // Successfully stopped - now delete the component_id -> function_class mapping
                        self.actor_instance_runtime_map.remove(&stop_function_id);
                        tracing::info!("Stopped actor: {stop_function_id}.");
                    }
                    Err(err) => {
                        tracing::error!("Unhandled stop actor error: {err}");
                        return;
                    }
                }
            }
            None => {
                tracing::error!("Could not find runner for {function_class}");
                return;
            }
        }
    }

    async fn patch_actor(&mut self, update: edgeless_api::common::PatchRequest) {
        tracing::debug!("Patch actor: {update:?}");

        // Get function class by looking it up in the instanceId->functionClass map
        let function_class: String = match self.actor_instance_runtime_map.get(&update.function_id) {
            Some(v) => v.clone(),
            None => {
                tracing::error!("Could not identify runtime hosting actor: {}.", update.function_id);
                return;
            }
        };

        // Get runner for function_class
        match self.actor_runtimes.iter_mut().find(|(k, _)| k.base_type == function_class) {
            Some((_, r)) => {
                // Forward the patch request to the correct runner
                match r.patch(update).await {
                    Ok(_) => {}
                    Err(err) => {
                        tracing::error!("Unhandled Patch Error: {err}");
                    }
                }
            }
            None => {
                tracing::error!("Could not find runner for {function_class}");
                return;
            }
        }
    }

    async fn start_resource(
        &mut self,
        instance_specification: edgeless_api::resource_configuration::ResourceInstanceSpecification,
    ) -> Result<edgeless_api::common::StartComponentResponse<edgeless_api::function_instance::InstanceId>, anyhow::Error> {
        if let Some(resource_desc) = self.resource_providers.get_mut(&instance_specification.provider_id) {
            let provider_id = instance_specification.provider_id.clone();

            if resource_desc.class_type != instance_specification.class_type {
                tracing::warn!("Resource Spawn request has invalid class type. Ignoring.");
            }

            let res = match resource_desc.client.start(instance_specification).await {
                Ok(val) => val,
                Err(err) => {
                    return Err(anyhow::anyhow!("Internal Resource Error {}", err));
                }
            };
            if let edgeless_api::common::StartComponentResponse::InstanceId(id) = res {
                tracing::info!(
                    "Started resource: class_type {}, provider_id {}, node_id {}, fid {}.",
                    resource_desc.class_type,
                    provider_id,
                    id.node_id,
                    id.function_id
                );
                self.resource_instance_provider_map.insert(id, provider_id);
                return Ok(edgeless_api::common::StartComponentResponse::InstanceId(id));
            } else {
                return Ok(res);
            }
        } else {
            return Ok(edgeless_api::common::StartComponentResponse::ResponseError(
                edgeless_api::common::ResponseError {
                    summary: "Error when creating a resource".to_string(),
                    detail: Some(format!("Provider for class_type does not exist: {}", instance_specification.class_type)),
                },
            ));
        }
    }

    async fn stop_resource(&mut self, resource_id: edgeless_api::function_instance::InstanceId) -> Result<(), anyhow::Error> {
        if let Some(provider_id) = self.resource_instance_provider_map.get(&resource_id) {
            if let Some(resource_desc) = self.resource_providers.get_mut(provider_id) {
                tracing::info!(
                    "Stopped resource: class_type {}, provider_id {} node_id {}, fid {}.",
                    resource_desc.class_type,
                    provider_id,
                    resource_id.node_id,
                    resource_id.function_id
                );
                return resource_desc.client.stop(resource_id).await;
            } else {
                return Err(anyhow::anyhow!(
                    "Cannot stop a resource, provider not found with provider_id: {}",
                    provider_id
                ));
            }
        }
        return Err(anyhow::anyhow!("Cannot stop a resource, not found with fid: {}", resource_id.function_id));
    }

    async fn patch_resource(&mut self, update: edgeless_api::common::PatchRequest) -> Result<(), anyhow::Error> {
        if let Some(provider_id) = self.resource_instance_provider_map.get(&update.function_id) {
            if let Some(resource_desc) = self.resource_providers.get_mut(provider_id) {
                tracing::info!("Patch resource: provider_id {} fid {}", provider_id, update.function_id);
                return resource_desc.client.patch(update).await;
            } else {
                return Err(anyhow::anyhow!(
                    "Cannot patch a resource, no provider found with provider_id: {}",
                    provider_id
                ));
            }
        }
        return Err(anyhow::anyhow!("Cannot patch a resource, not found with fid: {}", update.function_id));
    }

    async fn healt_status(&mut self) -> Result<edgeless_api::node_management::HealthStatus, anyhow::Error> {
        self.last_keepalive_timestamp = Some(tokio::time::Instant::now());

        // Refresh system/process information.
        self.sysinfo.refresh_cpu_all();
        self.sysinfo.refresh_memory();
        self.sysinfo.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[self.main_pid]), true);

        let to_kb = |x| (x / 1024) as i32;
        let proc = self.sysinfo.process(self.main_pid).unwrap();
        return Ok(edgeless_api::node_management::HealthStatus {
            cpu_usage: self.sysinfo.global_cpu_usage() as i32,
            cpu_load: self.sysinfo.cpus().iter().map(|x| x.cpu_usage() / 100_f32).sum::<f32>() as i32,
            mem_free: to_kb(self.sysinfo.free_memory()),
            mem_used: to_kb(self.sysinfo.used_memory()),
            mem_total: to_kb(self.sysinfo.total_memory()),
            mem_available: to_kb(self.sysinfo.available_memory()),
            proc_cpu_usage: proc.cpu_usage() as i32,
            proc_memory: to_kb(proc.memory()),
            proc_vmemory: to_kb(proc.virtual_memory()),
        });
    }

    async fn update_peers(&mut self, request: edgeless_api::node_management::UpdatePeersRequest) {
        tracing::debug!("Agent UpdatePeers {request:?}");
        match request {
            edgeless_api::node_management::UpdatePeersRequest::Add(node_id, invocation_url) => {
                self.dataplane_provider
                    .add_peer(EdgelessDataplanePeerSettings { node_id, invocation_url })
                    .await
            }
            edgeless_api::node_management::UpdatePeersRequest::Del(node_id) => self.dataplane_provider.del_peer(node_id).await,
            edgeless_api::node_management::UpdatePeersRequest::Clear => panic!("UpdatePeersRequest::Clear not implemented"),
        };
    }
}

#[derive(Clone)]
pub struct FunctionInstanceNodeClient {
    sender: futures::channel::mpsc::UnboundedSender<AgentRequest>,
}

#[derive(Clone)]
pub struct NodeManagementClient {
    sender: futures::channel::mpsc::UnboundedSender<AgentRequest>,
}

#[derive(Clone)]
pub struct ResourceConfigurationClient {
    sender: futures::channel::mpsc::UnboundedSender<AgentRequest>,
}
#[derive(Clone)]
pub struct LinkInstanceAPIClient {
    sender: futures::channel::mpsc::UnboundedSender<AgentRequest>,
}

#[derive(Clone)]
pub struct ProxyInstanceAPIClient {
    sender: futures::channel::mpsc::UnboundedSender<AgentRequest>,
}

#[derive(Clone)]
pub struct AgentClient {
    function_instance_client: Box<dyn edgeless_api::function_instance::FunctionInstanceAPI<edgeless_api::function_instance::InstanceId>>,
    node_management_client: Box<dyn edgeless_api::node_management::NodeManagementAPI>,
    resource_configuration_client:
        Box<dyn edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::InstanceId>>,
    link_instance_client: Box<dyn edgeless_api::link::LinkInstanceAPI>,
    proxy_instance_client: Box<dyn edgeless_api::proxy_instance::ProxyInstanceAPI>,
}

#[async_trait::async_trait]
impl edgeless_api::function_instance::FunctionInstanceAPI<edgeless_api::function_instance::InstanceId> for FunctionInstanceNodeClient {
    async fn start(
        &mut self,
        request: edgeless_api::function_instance::SpawnFunctionRequest,
    ) -> anyhow::Result<edgeless_api::common::StartComponentResponse<edgeless_api::function_instance::InstanceId>> {
        let f_id = request.instance_id;
        match self.sender.send(AgentRequest::Spawn(Box::new(request))).await {
            Ok(_) => Ok(edgeless_api::common::StartComponentResponse::InstanceId(f_id)),
            Err(err) => Err(anyhow::anyhow!(
                "Agent channel error when creating a function instance: {}",
                err.to_string()
            )),
        }
    }
    async fn stop(&mut self, id: edgeless_api::function_instance::InstanceId) -> anyhow::Result<()> {
        match self.sender.send(AgentRequest::Stop(id)).await {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!(
                "Agent channel error when stopping a function instance: {}",
                err.to_string()
            )),
        }
    }

    async fn patch(&mut self, update: edgeless_api::common::PatchRequest) -> anyhow::Result<()> {
        match self.sender.send(AgentRequest::Patch(update)).await {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!(
                "Agent channel error when updating the links of a function instance: {}",
                err.to_string()
            )),
        }
    }
}

#[async_trait::async_trait]
impl edgeless_api::node_management::NodeManagementAPI for NodeManagementClient {
    async fn update_peers(&mut self, request: edgeless_api::node_management::UpdatePeersRequest) -> anyhow::Result<()> {
        match self.sender.send(AgentRequest::UpdatePeers(request)).await {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!(
                "Agent channel error when updating the peers of a node: {}",
                err.to_string()
            )),
        }
    }

    async fn keep_alive(&mut self) -> anyhow::Result<edgeless_api::node_management::HealthStatus> {
        let (rsp_sender, rsp_receiver) = futures::channel::oneshot::channel::<anyhow::Result<edgeless_api::node_management::HealthStatus>>();
        let _ = self
            .sender
            .send(AgentRequest::HealthStatus(rsp_sender))
            .await
            .map_err(|err| anyhow::anyhow!("Agent channel error when querying health status: {}", err.to_string()))?;
        rsp_receiver
            .await
            .map_err(|err| anyhow::anyhow!("Agent channel error when querying health status: {}", err.to_string()))?
    }
}

#[async_trait::async_trait]
impl edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::InstanceId> for ResourceConfigurationClient {
    async fn start(
        &mut self,
        request: edgeless_api::resource_configuration::ResourceInstanceSpecification,
    ) -> anyhow::Result<edgeless_api::common::StartComponentResponse<edgeless_api::function_instance::InstanceId>> {
        let (rsp_sender, rsp_receiver) = futures::channel::oneshot::channel::<
            anyhow::Result<edgeless_api::common::StartComponentResponse<edgeless_api::function_instance::InstanceId>>,
        >();
        let _ = self
            .sender
            .send(AgentRequest::SpawnResource(request, rsp_sender))
            .await
            .map_err(|err| anyhow::anyhow!("Agent channel error when creating a resource instance: {}", err.to_string()))?;
        rsp_receiver
            .await
            .map_err(|err| anyhow::anyhow!("Agent channel error when creating a resource instance: {}", err.to_string()))?
    }
    async fn stop(&mut self, id: edgeless_api::function_instance::InstanceId) -> anyhow::Result<()> {
        let (rsp_sender, rsp_receiver) = futures::channel::oneshot::channel::<anyhow::Result<()>>();
        self.sender
            .send(AgentRequest::StopResource(id, rsp_sender))
            .await
            .map_err(|err| anyhow::anyhow!("Agent channel error when creating a resource instance: {}", err.to_string()))?;
        rsp_receiver
            .await
            .map_err(|err| anyhow::anyhow!("Agent channel error when creating a resource instance: {}", err.to_string()))?
    }

    async fn patch(&mut self, update: edgeless_api::common::PatchRequest) -> anyhow::Result<()> {
        let (rsp_sender, rsp_receiver) = futures::channel::oneshot::channel::<anyhow::Result<()>>();
        self.sender
            .send(AgentRequest::PatchResource(update, rsp_sender))
            .await
            .map_err(|err| anyhow::anyhow!("Agent channel error when creating a resource instance: {}", err.to_string()))?;
        rsp_receiver
            .await
            .map_err(|err| anyhow::anyhow!("Agent channel error when creating a resource instance: {}", err.to_string()))?
    }
}

#[async_trait::async_trait]
impl edgeless_api::link::LinkInstanceAPI for LinkInstanceAPIClient {
    async fn create(&mut self, req: edgeless_api::link::CreateLinkRequest) -> anyhow::Result<()> {
        self.sender
            .send(AgentRequest::CreateLink(req))
            .await
            .map_err(|err| anyhow::anyhow!("Agent channel error when creating a link: {}", err.to_string()))?;
        Ok(())
    }
    async fn remove(&mut self, id: edgeless_api::link::LinkInstanceId) -> anyhow::Result<()> {
        self.sender
            .send(AgentRequest::RemoveLink(id))
            .await
            .map_err(|err| anyhow::anyhow!("Agent channel error when  removing a link: {}", err.to_string()))?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl edgeless_api::proxy_instance::ProxyInstanceAPI for ProxyInstanceAPIClient {
    async fn start(&mut self, request: edgeless_api::proxy_instance::ProxySpec) -> anyhow::Result<()> {
        match self.sender.send(AgentRequest::StartProxy(request)).await {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!("Agent channel error: {}", err.to_string())),
        }
    }
    async fn stop(&mut self, id: edgeless_api::function_instance::InstanceId) -> anyhow::Result<()> {
        match self.sender.send(AgentRequest::StopProxy(id)).await {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!("Agent channel error: {}", err.to_string())),
        }
    }
    async fn patch(&mut self, update: edgeless_api::proxy_instance::ProxySpec) -> anyhow::Result<()> {
        match self.sender.send(AgentRequest::PatchProxy(update)).await {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!("Agent channel error: {}", err.to_string())),
        }
    }
}

impl edgeless_api::agent::AgentAPI for AgentClient {
    fn function_instance_api(
        &mut self,
    ) -> Box<dyn edgeless_api::function_instance::FunctionInstanceAPI<edgeless_api::function_instance::InstanceId>> {
        self.function_instance_client.clone()
    }

    fn node_management_api(&mut self) -> Box<dyn edgeless_api::node_management::NodeManagementAPI> {
        self.node_management_client.clone()
    }

    fn resource_configuration_api(
        &mut self,
    ) -> Box<dyn edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::InstanceId>> {
        self.resource_configuration_client.clone()
    }

    fn link_instance_api(&mut self) -> Box<dyn edgeless_api::link::LinkInstanceAPI> {
        self.link_instance_client.clone()
    }

    fn proxy_instance_api(&mut self) -> Box<dyn edgeless_api::proxy_instance::ProxyInstanceAPI> {
        self.proxy_instance_client.clone()
    }
}
