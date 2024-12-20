// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

// This contains code originally developed in edgeless_orc (also in some of the related files).
// Refer to the orchestrator's history for the history/authorship of those snippets.

use edgeless_api::common::ResponseError;
use futures::StreamExt;

use crate::ir::RequiredChange;

pub struct ControllerTask {
    request_receiver: futures::channel::mpsc::UnboundedReceiver<super::ControllerRequest>,
    cluster_id: edgeless_api::function_instance::NodeId,
    nodes: std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<edgeless_api::function_instance::NodeId, WorkerNode>>>,
    peer_clusters: std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<edgeless_api::function_instance::NodeId, PeerCluster>>>,
    link_controllers:
        std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<edgeless_api::link::LinkType, Box<dyn edgeless_api::link::LinkController>>>>,
    active_workflows: std::collections::HashMap<edgeless_api::workflow_instance::WorkflowId, super::super::ir::managed_worflow::ManagedWorkflow>,
    orchestration_logic: std::sync::Arc<tokio::sync::Mutex<crate::orchestration_logic::OrchestrationLogic>>,
}
pub struct WorkerNode {
    pub agent_url: String,
    pub invocation_url: String,
    pub api: Box<dyn edgeless_api::agent::AgentAPI + Send>,
    pub resource_providers: std::collections::HashMap<String, ResourceProvider>,
    pub capabilities: edgeless_api::node_registration::NodeCapabilities,
    pub health_status: edgeless_api::node_management::HealthStatus,
    pub weight: f32,
    pub supported_link_types: std::collections::HashMap<edgeless_api::link::LinkType, edgeless_api::link::LinkProviderId>,
    // This should probably be based on link types and is a placeholder
    pub is_proxy: bool,
}

pub struct PeerCluster {
    pub controller_url: String,
    pub api: Box<dyn edgeless_api::controller::ControllerAPI + Send>,
    pub supported_link_types: std::collections::HashMap<edgeless_api::link::LinkType, edgeless_api::link::LinkProviderId>,
}

#[derive(serde::Serialize)]
pub struct ResourceProvider {
    pub class_type: String,
    pub outputs: Vec<String>,
}

impl ControllerTask {
    pub fn new(
        cluster_id: edgeless_api::function_instance::NodeId,
        request_receiver: futures::channel::mpsc::UnboundedReceiver<super::ControllerRequest>,
    ) -> Self {
        Self {
            request_receiver,
            nodes: std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
            cluster_id,
            peer_clusters: std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
            link_controllers: std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::from([(
                edgeless_api::link::LinkType("MULTICAST".to_string()),
                Box::new(edgeless_link_multicast::controller::MulticastController::new()) as Box<dyn edgeless_api::link::LinkController>,
            )]))),
            active_workflows: std::collections::HashMap::new(),
            orchestration_logic: std::sync::Arc::new(tokio::sync::Mutex::new(crate::orchestration_logic::OrchestrationLogic::new(
                crate::orchestration_utils::OrchestrationStrategy::Random,
            ))),
        }
    }

    pub async fn run(&mut self) {
        self.main_loop().await;
    }

    async fn main_loop(&mut self) {
        let mut check_interval = tokio::time::interval(tokio::time::Duration::from_secs(2));
        loop {
            tokio::select! {
                req = self.request_receiver.next() => {
                    if let Some(req) = req {
                        match req {
                            super::ControllerRequest::START(spawn_workflow_request, reply_sender) => {
                                // log::info!("{:?}", spawn_workflow_request);
                                let reply = self.start_workflow(spawn_workflow_request).await;
                                match reply_sender.send(reply) {
                                    Ok(_) => {}
                                    Err(err) => {
                                        log::error!("Unhandled: {:?}", err);
                                    }
                                }
                            }
                            super::ControllerRequest::STOP(wf_id) => {
                                self.stop_workflow(&wf_id).await;
                            }
                            super::ControllerRequest::LIST(workflow_id, reply_sender) => {
                                let reply = self.list_workflows(&workflow_id).await;
                                match reply_sender.send(reply) {
                                    Ok(_) => {}
                                    Err(err) => {
                                        log::error!("Unhandled: {:?}", err);
                                    }
                                }
                            }
                            super::ControllerRequest::UPDATENODE(update, reply_sender) => {
                                let reply = match update {
                                    edgeless_api::node_registration::UpdateNodeRequest::Registration(node_id, agent_url, invocation_url, resource_providers, capabilities, link_providers) => self.process_node_registration(node_id, agent_url, invocation_url, resource_providers, capabilities, link_providers).await,
                                    edgeless_api::node_registration::UpdateNodeRequest::Deregistration(node_id) => self.process_node_del(node_id).await,
                                };
                                match reply_sender.send(reply) {
                                    Ok(_) => {}
                                    Err(err) => {
                                        log::error!("Unhandled: {:?}", err);
                                    }
                                }
                            },
                            super::ControllerRequest::PATCH(update) => {
                                let _res = self.patch_workflow(&update).await;
                            }
                        }
                    }
                },
                _ = check_interval.tick() => {
                    self.periodic_health_check().await;
                }

            }
        }
    }

    async fn start_workflow(
        &mut self,
        spawn_workflow_request: edgeless_api::workflow_instance::SpawnWorkflowRequest,
    ) -> anyhow::Result<edgeless_api::workflow_instance::SpawnWorkflowResponse> {
        // Assign a new identifier to the newly-created workflow.
        let wf_id = edgeless_api::workflow_instance::WorkflowId {
            workflow_id: uuid::Uuid::new_v4(),
        };

        let mut wf = super::super::ir::managed_worflow::ManagedWorkflow::new(
            spawn_workflow_request.clone(),
            wf_id.clone(),
            self.orchestration_logic.clone(),
            self.nodes.clone(),
            self.peer_clusters.clone(),
            self.link_controllers.clone(),
        );
        let required_changes = tokio::task::block_in_place(|| wf.initial_spawn());

        let desc = edgeless_api::workflow_instance::WorkflowInstance {
            workflow_id: wf_id.clone(),
            node_mapping: wf
                .wf
                .components()
                .iter()
                .filter_map(|(id, a)| {
                    let instances: Vec<_> = a.borrow_mut().instance_ids().iter().map(|i| i.node_id.to_string()).collect();
                    if !instances.is_empty() {
                        Some(edgeless_api::workflow_instance::WorkflowFunctionMapping {
                            name: id.to_string(),
                            node_ids: instances,
                        })
                    } else {
                        None
                    }
                })
                .collect(),
        };

        self.active_workflows.insert(wf_id.clone(), wf);

        let res = self.materialize(wf_id.clone(), required_changes).await;

        if res.is_err() {
            self.stop_workflow(&wf_id).await;
        }

        match res {
            Ok(_) => Ok(edgeless_api::workflow_instance::SpawnWorkflowResponse::WorkflowInstance(desc)),
            Err(err) => Ok(edgeless_api::workflow_instance::SpawnWorkflowResponse::ResponseError(
                edgeless_api::common::ResponseError {
                    summary: "Workflow creation failed".to_string(),
                    detail: Some(err.join(";")),
                },
            )),
        }
    }

    async fn stop_workflow(&mut self, wf_id: &edgeless_api::workflow_instance::WorkflowId) {
        let mut workflow = match self.active_workflows.remove(wf_id) {
            None => {
                log::error!("trying to tear-down a workflow that does not exist: {}", wf_id.to_string());
                return;
            }
            Some(val) => val,
        };

        let changes = workflow.stop();
        if let Err(errs) = self.materialize(wf_id.clone(), changes).await {
            log::info!("Failures while stopping workflow: {}", errs.join(";"));
        };
    }

    async fn list_workflows(
        &mut self,
        workflow_id: &edgeless_api::workflow_instance::WorkflowId,
    ) -> anyhow::Result<Vec<edgeless_api::workflow_instance::WorkflowInstance>> {
        let mut ret: Vec<edgeless_api::workflow_instance::WorkflowInstance> = vec![];
        if let Some(wf) = self.active_workflows.get(workflow_id) {
            ret = vec![edgeless_api::workflow_instance::WorkflowInstance {
                workflow_id: workflow_id.clone(),
                node_mapping: wf
                    .wf
                    .components()
                    .iter()
                    .map(|(id, a)| edgeless_api::workflow_instance::WorkflowFunctionMapping {
                        name: id.to_string(),
                        node_ids: a.borrow_mut().instance_ids().iter().map(|i| i.node_id.to_string()).collect(),
                    })
                    .collect(),
            }];
        } else {
            ret = self
                .active_workflows
                .iter()
                .map(|(wf_id, wf)| edgeless_api::workflow_instance::WorkflowInstance {
                    workflow_id: wf_id.clone(),
                    node_mapping: wf
                        .wf
                        .components()
                        .iter()
                        .map(|(id, a)| edgeless_api::workflow_instance::WorkflowFunctionMapping {
                            name: id.to_string(),
                            node_ids: a.borrow_mut().instance_ids().iter().map(|i| i.node_id.to_string()).collect(),
                        })
                        .collect(),
                })
                .collect();
        }
        Ok(ret)
    }

    async fn patch_workflow(&mut self, req: &edgeless_api::common::PatchRequest) -> anyhow::Result<()> {
        let id = edgeless_api::workflow_instance::WorkflowId {
            workflow_id: req.function_id.function_id,
        };
        if let Some(wf) = self.active_workflows.get_mut(&id) {
            let required_changes = tokio::task::block_in_place(|| wf.patch_external_links(req.clone()));
            if let Err(errs) = self.materialize(id.clone(), required_changes).await {
                log::info!("Failures while stopping workflow: {}", errs.join(";"));
            };
        }
        Ok(())
    }

    async fn process_node_registration(
        &mut self,
        node_id: uuid::Uuid,
        agent_url: String,
        invocation_url: String,
        resource_providers: Vec<edgeless_api::node_registration::ResourceProviderSpecification>,
        capabilities: edgeless_api::node_registration::NodeCapabilities,
        link_providers: Vec<edgeless_api::node_registration::LinkProviderSpecification>,
    ) -> anyhow::Result<edgeless_api::node_registration::UpdateNodeResponse> {
        if let Some(node) = self.nodes.lock().await.get(&node_id) {
            if node.agent_url == agent_url && node.invocation_url == invocation_url {
                return Ok(edgeless_api::node_registration::UpdateNodeResponse::Accepted);
            } else {
                return Ok(edgeless_api::node_registration::UpdateNodeResponse::ResponseError(ResponseError {
                    summary: "Duplicate NodeId with different URL(s).".to_string(),
                    detail: None,
                }));
            }
        }

        let api = Self::get_api_for_url(&agent_url).await;

        let mut node_weight = (std::cmp::max(capabilities.num_cores, capabilities.num_cpus) as f32) * capabilities.clock_freq_cpu;
        if node_weight == 0.0 {
            // Force a vanishing weight to an arbitrary value.
            node_weight = 1.0;
        };

        self.nodes.lock().await.insert(
            node_id,
            WorkerNode {
                agent_url,
                invocation_url: invocation_url.clone(),
                api,
                resource_providers: resource_providers
                    .into_iter()
                    .map(|r| {
                        (
                            r.provider_id,
                            ResourceProvider {
                                class_type: r.class_type,
                                outputs: r.outputs,
                            },
                        )
                    })
                    .collect(),
                capabilities,
                health_status: edgeless_api::node_management::HealthStatus::empty(),
                weight: node_weight,
                supported_link_types: link_providers.into_iter().map(|p| (p.class, p.provider_id)).collect(),
                is_proxy: true,
            },
        );

        self.send_peer_updates(vec![edgeless_api::node_management::UpdatePeersRequest::Add(node_id, invocation_url)])
            .await;

        // Send information about all nodes to the new node.
        let updates: Vec<_> = self
            .nodes
            .lock()
            .await
            .iter()
            .filter_map(|(n_id, n_spec)| {
                if n_id != &node_id {
                    Some(edgeless_api::node_management::UpdatePeersRequest::Add(
                        *n_id,
                        n_spec.invocation_url.clone(),
                    ))
                } else {
                    None
                }
            })
            .collect();
        {
            // let n = self.nodes.borrow_mut().get_mut(&node_id).unwrap();
            if let Some(node) = self.nodes.lock().await.get_mut(&node_id) {
                for update in updates {
                    node.api.node_management_api().update_peers(update).await.unwrap();
                }
            }
        }

        Ok(edgeless_api::node_registration::UpdateNodeResponse::Accepted)
    }

    async fn process_node_del(
        &mut self,
        node_id: edgeless_api::function_instance::NodeId,
    ) -> anyhow::Result<edgeless_api::node_registration::UpdateNodeResponse> {
        let old_value = self.nodes.lock().await.remove(&node_id);
        if old_value.is_some() {
            self.handle_node_removal(&std::collections::HashSet::from_iter(vec![node_id].into_iter()))
                .await;
            self.send_peer_updates(vec![edgeless_api::node_management::UpdatePeersRequest::Del(node_id)])
                .await;
            Ok(edgeless_api::node_registration::UpdateNodeResponse::Accepted)
        } else {
            Ok(edgeless_api::node_registration::UpdateNodeResponse::Accepted)
        }
    }

    async fn materialize(
        &mut self,
        wf_id: edgeless_api::workflow_instance::WorkflowId,
        required_changes: Vec<RequiredChange>,
    ) -> Result<(), Vec<String>> {
        let mut results = Vec::<Result<(), String>>::new();

        // This could be parallel
        for f in required_changes.into_iter() {
            results.push(match f {
                RequiredChange::StartFunction {
                    function_id,
                    image,
                    input_mapping,
                    output_mapping,
                    function_name,
                    annotations,
                } => {
                    self.start_workflow_function_on_node(&wf_id, function_name, function_id, image, input_mapping, output_mapping, annotations)
                        .await
                }
                RequiredChange::StartResource {
                    resource_id,
                    resource_name,
                    class_type,
                    input_mapping,
                    output_mapping,
                    configuration,
                } => {
                    self.start_workflow_resource_on_node(
                        &wf_id,
                        resource_name,
                        resource_id,
                        class_type,
                        output_mapping,
                        input_mapping,
                        configuration,
                    )
                    .await
                }
                RequiredChange::PatchFunction {
                    function_id,
                    function_name,
                    input_mapping,
                    output_mapping,
                } => {
                    self.patch_outputs(function_id, super::ComponentType::Function, output_mapping, input_mapping, &function_name)
                        .await
                }
                RequiredChange::PatchResource {
                    resource_id,
                    resource_name,
                    input_mapping,
                    output_mapping,
                } => {
                    self.patch_outputs(resource_id, super::ComponentType::Resource, output_mapping, input_mapping, &resource_name)
                        .await
                }
                RequiredChange::InstantiateLinkControlPlane { link_id, class } => self.create_link_control_plane(link_id, class).await,
                RequiredChange::CreateLinkOnNode {
                    link_id,
                    node_id,
                    config,
                    provider_id,
                } => self.create_link_on_node(link_id, node_id, provider_id, config).await,
                RequiredChange::RemoveLinkFromNode { link_id, node_id } => self.remove_link_from_node(link_id, node_id).await,
                RequiredChange::CreateSubflow { subflow_id, spawn_req } => self.start_subflow_on_cluster(subflow_id, spawn_req).await,
                RequiredChange::PatchSubflow {
                    subflow_id,
                    input_mapping,
                    output_mapping,
                } => {
                    self.patch_outputs(subflow_id, super::ComponentType::SubFlow, output_mapping, input_mapping, "subflow")
                        .await
                }
                RequiredChange::PatchProxy {
                    proxy_id,
                    internal_inputs,
                    internal_outputs,
                    external_inputs,
                    external_outputs,
                } => {
                    self.patch_proxy_instance(proxy_id, internal_inputs, internal_outputs, external_inputs, external_outputs)
                        .await
                }
                RequiredChange::CrateProxy {
                    proxy_id,
                    internal_inputs,
                    internal_outputs,
                    external_inputs,
                    external_outputs,
                } => {
                    self.start_proxy_on_node(proxy_id, internal_inputs, internal_outputs, external_inputs, external_outputs)
                        .await
                }
            });
        }

        let mut error_msg = Vec::new();
        for res in results {
            if let Err(msg) = res {
                error_msg.push(msg);
            }
        }

        if error_msg.is_empty() {
            Ok(())
        } else {
            Err(error_msg)
        }
    }

    async fn start_workflow_function_on_node(
        &mut self,
        wf_id: &edgeless_api::workflow_instance::WorkflowId,
        f_name: String,
        function_id: edgeless_api::function_instance::InstanceId,
        image: super::super::ir::actor::ActorImage,
        input_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, super::super::ir::PhysicalInput>,
        output_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, super::super::ir::PhysicalOutput>,
        annotations: std::collections::HashMap<String, String>,
    ) -> Result<(), String> {
        // [TODO] Issue#95
        // The state_specification configuration should be
        // read from the function annotations.
        log::debug!("state specifications currently forced to NodeLocal");
        log::info!("{:?}", output_mapping);
        let response = self
            .fn_client(&function_id.node_id)
            .await
            .ok_or(format!("No function client for node: {}", &function_id.node_id))?
            .start(edgeless_api::function_instance::SpawnFunctionRequest {
                instance_id: function_id,
                code: edgeless_api::function_instance::FunctionClassSpecification {
                    function_class_id: image.class.id.id.clone(),
                    function_class_type: image.format.clone(),
                    function_class_version: image.class.id.version.clone(),
                    function_class_code: image.code.clone(),
                    function_class_outputs: image.class.outputs.clone(),
                    function_class_inputs: image.class.inputs.clone(),
                    function_class_inner_structure: image
                        .class
                        .inner_structure
                        .iter()
                        .map(|(src, dst)| (src.clone(), dst.clone().into_iter().collect()))
                        .collect(),
                },
                annotations: annotations.clone(),
                state_specification: edgeless_api::function_instance::StateSpecification {
                    state_id: uuid::Uuid::new_v4(),
                    state_policy: edgeless_api::function_instance::StatePolicy::NodeLocal,
                },
                input_mapping: input_mapping.clone(),
                output_mapping: output_mapping.clone(),
            })
            .await;

        match response {
            Ok(response) => match response {
                edgeless_api::common::StartComponentResponse::ResponseError(error) => {
                    log::warn!("function instance creation rejected: {}", error);
                    Err(format!("function instance creation rejected: {} ", error))
                }
                edgeless_api::common::StartComponentResponse::InstanceId(id) => Ok(()),
            },
            Err(err) => Err(format!("failed interaction when creating a function instance: {}", err)),
        }
    }

    async fn start_workflow_resource_on_node(
        &mut self,
        wf_id: &edgeless_api::workflow_instance::WorkflowId,
        r_name: String,
        resource_id: edgeless_api::function_instance::InstanceId,
        class_type: String,
        output_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, super::super::ir::PhysicalOutput>,
        input_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, super::super::ir::PhysicalInput>,
        configurations: std::collections::HashMap<String, String>,
    ) -> Result<(), String> {
        let response = self
            .resource_client(&resource_id.node_id)
            .await
            .ok_or(format!("No resource client for node: {}", &resource_id.node_id))?
            .start(edgeless_api::resource_configuration::ResourceInstanceSpecification {
                resource_id,
                class_type: class_type.clone(),
                configuration: configurations.clone(),
                output_mapping: output_mapping.clone(),
                input_mapping: input_mapping.clone(),
            })
            .await;

        match response {
            Ok(response) => match response {
                edgeless_api::common::StartComponentResponse::ResponseError(error) => {
                    log::warn!("resource start rejected: {}", error);
                    Err(format!("resource start rejected: {} ", error))
                }
                edgeless_api::common::StartComponentResponse::InstanceId(id) => {
                    log::info!("workflow {} resource {} started with fid {}", wf_id.to_string(), &r_name, &id);
                    Ok(())
                }
            },
            Err(err) => Err(format!("failed interaction when starting a resource: {}", err)),
        }
    }

    async fn start_subflow_on_cluster(
        &mut self,
        subflow_id: edgeless_api::function_instance::InstanceId,
        spawn_req: edgeless_api::workflow_instance::SpawnWorkflowRequest,
    ) -> Result<(), String> {
        if let Some(cluster) = self.peer_clusters.lock().await.get_mut(&subflow_id.node_id) {
            cluster.api.workflow_instance_api().start(spawn_req).await.map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Err("Failed to start subflow".to_string())
        }
    }

    async fn start_proxy_on_node(
        &mut self,
        proxy_id: edgeless_api::function_instance::InstanceId,
        internal_inputs: std::collections::HashMap<edgeless_api::function_instance::PortId, edgeless_api::common::Input>,
        internal_outputs: std::collections::HashMap<edgeless_api::function_instance::PortId, edgeless_api::common::Output>,
        external_inputs: std::collections::HashMap<edgeless_api::function_instance::PortId, edgeless_api::common::Input>,
        external_outputs: std::collections::HashMap<edgeless_api::function_instance::PortId, edgeless_api::common::Output>,
    ) -> Result<(), String> {
        match self
            .proxy_client(&proxy_id.node_id)
            .await
            .ok_or(format!("No proxy client for node {}", proxy_id.node_id))?
            .start(edgeless_api::proxy_instance::ProxySpec {
                instance_id: proxy_id,
                inner_outputs: internal_outputs,
                inner_inputs: internal_inputs,
                external_outputs,
                external_inputs,
            })
            .await
        {
            Ok(_) => Ok(()),
            Err(err) => Err(format!("failed starting proxy: {}", err)),
        }
    }

    async fn periodic_health_check(&mut self) {
        // First check if there are nodes that must be disconnected
        // because they failed to reply to a keep-alive.
        let to_be_disconnected = self.find_dead_nodes().await;

        // Second, remove all those nodes from the map of clients.
        for node_id in to_be_disconnected.iter() {
            log::info!("disconnected node not replying to keep-alive: {}", &node_id);
            let val = self.nodes.lock().await.remove(node_id);
            assert!(val.is_some());
        }

        // Update the peers of (still alive) nodes by
        // deleting the missing-in-action peers.
        for removed_node_id in &to_be_disconnected {
            for (_, client_desc) in self.nodes.lock().await.iter_mut() {
                match client_desc
                    .api
                    .node_management_api()
                    .update_peers(edgeless_api::node_management::UpdatePeersRequest::Del(*removed_node_id))
                    .await
                {
                    Ok(_) => {}
                    Err(err) => {
                        log::error!("Unhandled: {}", err);
                    }
                }
            }
        }

        self.handle_node_removal(&to_be_disconnected).await;
    }

    async fn find_dead_nodes(&mut self) -> std::collections::HashSet<edgeless_api::function_instance::NodeId> {
        let mut dead_nodes = std::collections::HashSet::new();
        for (node_id, client_desc) in self.nodes.lock().await.iter_mut() {
            match client_desc.api.node_management_api().keep_alive().await {
                Ok(health_status) => {
                    client_desc.health_status = health_status;
                }
                Err(_) => {
                    dead_nodes.insert(*node_id);
                }
            };
        }
        dead_nodes
    }

    async fn handle_node_removal(&mut self, removed_nodes: &std::collections::HashSet<edgeless_api::function_instance::NodeId>) {
        for wf_id in self
            .active_workflows
            .keys()
            .cloned()
            .collect::<Vec<edgeless_api::workflow_instance::WorkflowId>>()
        {
            self.handle_node_removal_for_workflow(removed_nodes, wf_id.clone()).await;
        }
    }

    async fn handle_node_removal_for_workflow(
        &mut self,
        removed_nodes: &std::collections::HashSet<edgeless_api::function_instance::NodeId>,
        wf_id: edgeless_api::workflow_instance::WorkflowId,
    ) {
        if let Some(wf) = self.active_workflows.get_mut(&wf_id) {
            let required_changes = tokio::task::block_in_place(|| wf.node_removal(removed_nodes));
            if let Err(errs) = self.materialize(wf_id, required_changes).await {
                log::error!("Failures Handling Node Removal: {}", errs.join(";"));
            }
        }
    }

    async fn patch_outputs(
        &mut self,
        origin_id: edgeless_api::function_instance::InstanceId,
        origin_type: super::ComponentType,
        output_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, edgeless_api::common::Output>,
        input_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, edgeless_api::common::Input>,
        name_in_workflow: &str,
    ) -> Result<(), String> {
        match origin_type {
            super::ComponentType::Function => {
                match self
                    .fn_client(&origin_id.node_id)
                    .await
                    .ok_or(format!("No function client for node: {}", origin_id.node_id))?
                    .patch(edgeless_api::common::PatchRequest {
                        function_id: origin_id,
                        output_mapping,
                        input_mapping,
                    })
                    .await
                {
                    Ok(_) => Ok(()),
                    Err(err) => Err(format!("failed interaction when patching component {}: {}", name_in_workflow, err)),
                }
            }
            super::ComponentType::Resource => {
                match self
                    .resource_client(&origin_id.node_id)
                    .await
                    .ok_or(format!("No resource client for node: {}", origin_id.node_id))?
                    .patch(edgeless_api::common::PatchRequest {
                        function_id: origin_id,
                        output_mapping,
                        input_mapping,
                    })
                    .await
                {
                    Ok(_) => Ok(()),
                    Err(err) => Err(format!("failed interaction when patching component {}: {}", name_in_workflow, err)),
                }
            }
            super::ComponentType::SubFlow => {
                match self
                    .workflow_client(&origin_id.node_id)
                    .await
                    .ok_or(format!("No workflow client for cluster {}", origin_id.node_id))?
                    .patch(edgeless_api::common::PatchRequest {
                        function_id: origin_id,
                        output_mapping,
                        input_mapping,
                    })
                    .await
                {
                    Ok(_) => Ok(()),
                    Err(err) => Err(format!("failed interaction when patching component {}: {}", name_in_workflow, err)),
                }
            }
        }
    }

    async fn patch_proxy_instance(
        &mut self,
        proxy_id: edgeless_api::function_instance::InstanceId,
        internal_inputs: std::collections::HashMap<edgeless_api::function_instance::PortId, edgeless_api::common::Input>,
        internal_outputs: std::collections::HashMap<edgeless_api::function_instance::PortId, edgeless_api::common::Output>,
        external_inputs: std::collections::HashMap<edgeless_api::function_instance::PortId, edgeless_api::common::Input>,
        external_outputs: std::collections::HashMap<edgeless_api::function_instance::PortId, edgeless_api::common::Output>,
    ) -> Result<(), String> {
        match self
            .proxy_client(&proxy_id.node_id)
            .await
            .ok_or(format!("No proxy client for node {}", proxy_id.node_id))?
            .patch(edgeless_api::proxy_instance::ProxySpec {
                instance_id: proxy_id,
                inner_outputs: internal_outputs,
                inner_inputs: internal_inputs,
                external_outputs,
                external_inputs,
            })
            .await
        {
            Ok(_) => Ok(()),
            Err(err) => Err(format!("failed patching proxy {}", err)),
        }
    }

    async fn create_link_control_plane(
        &mut self,
        link_id: edgeless_api::link::LinkInstanceId,
        class: edgeless_api::link::LinkType,
    ) -> Result<(), String> {
        if let Some(lc) = self.link_controllers.lock().await.get_mut(&class) {
            lc.instantiate_control_plane(link_id).await;
        }
        Ok(())
    }

    async fn create_link_on_node(
        &mut self,
        link_id: edgeless_api::link::LinkInstanceId,
        node_id: edgeless_api::function_instance::NodeId,
        link_provider_id: edgeless_api::link::LinkProviderId,
        config: Vec<u8>,
    ) -> Result<(), String> {
        if let Some(node) = self.nodes.lock().await.get_mut(&node_id) {
            node.api
                .link_instance_api()
                .create(edgeless_api::link::CreateLinkRequest {
                    id: link_id,
                    provider: link_provider_id,
                    config,
                    direction: edgeless_api::link::LinkDirection::BiDi,
                })
                .await
                .map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Err("Node Not Found".to_string())
        }
    }

    async fn remove_link_from_node(
        &mut self,
        link_id: edgeless_api::link::LinkInstanceId,
        node_id: edgeless_api::function_instance::NodeId,
    ) -> Result<(), String> {
        if let Some(node) = self.nodes.lock().await.get_mut(&node_id) {
            node.api.link_instance_api().remove(link_id).await.map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Err("Node Not Found".to_string())
        }
    }

    async fn fn_client(
        &mut self,
        node_id: &edgeless_api::function_instance::NodeId,
    ) -> Option<Box<dyn edgeless_api::function_instance::FunctionInstanceAPI<edgeless_api::function_instance::InstanceId>>> {
        Some(self.nodes.lock().await.get_mut(node_id)?.api.function_instance_api())
    }

    async fn resource_client(
        &mut self,
        node_id: &edgeless_api::function_instance::NodeId,
    ) -> Option<Box<dyn edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::InstanceId>>> {
        Some(self.nodes.lock().await.get_mut(node_id)?.api.resource_configuration_api())
    }

    async fn workflow_client(
        &mut self,
        cluster_id: &edgeless_api::function_instance::NodeId,
    ) -> Option<Box<dyn edgeless_api::workflow_instance::WorkflowInstanceAPI>> {
        Some(self.peer_clusters.lock().await.get_mut(cluster_id)?.api.workflow_instance_api())
    }

    async fn proxy_client(
        &mut self,
        node_id: &edgeless_api::function_instance::NodeId,
    ) -> Option<Box<dyn edgeless_api::proxy_instance::ProxyInstanceAPI>> {
        Some(self.nodes.lock().await.get_mut(node_id)?.api.proxy_instance_api())
    }

    async fn send_peer_updates(&mut self, updates: Vec<edgeless_api::node_management::UpdatePeersRequest>) {
        for (n_id, n_spec) in self.nodes.lock().await.iter_mut() {
            for update in &updates {
                let is_self = match update {
                    edgeless_api::node_management::UpdatePeersRequest::Add(id, _) => id == n_id,
                    edgeless_api::node_management::UpdatePeersRequest::Del(id) => id == n_id,
                    edgeless_api::node_management::UpdatePeersRequest::Clear => false,
                };

                if !is_self {
                    n_spec.api.node_management_api().update_peers(update.clone()).await.unwrap();
                }
            }
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
}
