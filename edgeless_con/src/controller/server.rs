// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-FileCopyrightText: © 2023 University of Cambridge, System Research Group
// SPDX-FileCopyrightText: © 2024 Roman Kolcun <roman.kolcun@cl.cam.ac.uk>
// SPDX-License-Identifier: MIT

// This contains code originally developed in edgeless_orc (also in some of the related files).
// Refer to the orchestrator's history for the history/authorship of those snippets.

use edgeless_api::common::ResponseError;
use futures::StreamExt;

pub struct ControllerTask<P: crate::ir::transformations::placement::strategy::PlacementStrategy> {
    request_receiver: futures::channel::mpsc::UnboundedReceiver<super::ControllerRequest>,
    cluster_id: edgeless_api::function_instance::NodeId,
    nodes: std::collections::HashMap<edgeless_api::function_instance::NodeId, super::node::WorkerNode>,
    peer_clusters: std::collections::HashMap<edgeless_api::function_instance::NodeId, super::peer_cluster::PeerCluster>,
    active_workflows: std::collections::HashMap<edgeless_api::workflow_instance::WorkflowId, super::workflow::WorkflowInstance>,
    telemetry_provider: Option<Box<dyn crate::ir::TelemetryProvider>>,
    image_repository: super::image_repository::ImageRepository,
    global_pipeline_state: std::sync::Arc<crate::ir::pipeline::default::DefaultTransformationPipelineState<P::GlobalState>>,
}

impl<P: crate::ir::transformations::placement::strategy::PlacementStrategy + 'static> ControllerTask<P> {
    pub fn new(
        cluster_id: edgeless_api::function_instance::NodeId,
        request_receiver: futures::channel::mpsc::UnboundedReceiver<super::ControllerRequest>,
        telemetry_provider: Option<Box<dyn crate::ir::TelemetryProvider>>,
        image_repository: super::image_repository::ImageRepository,
    ) -> Self {
        let global_pipeline_state = crate::ir::pipeline::default::DefaultTransformationPipelineState::<P::GlobalState> {
            placement_strategy_state: P::GlobalState::default(),
            interaction_dialect_registry: std::sync::Arc::new(tokio::sync::Mutex::new(
                crate::ir::interaction::dialect::DialectRegistry::new_default(),
            )),
            image_cache: crate::ir::support::image_cache::ImageCache::default(),
            instance_counts: Default::default(),
        };

        Self {
            request_receiver,
            nodes: std::collections::HashMap::new(),
            cluster_id,
            peer_clusters: std::collections::HashMap::new(),
            active_workflows: std::collections::HashMap::new(),
            telemetry_provider,
            image_repository,
            global_pipeline_state: std::sync::Arc::new(global_pipeline_state),
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
                            super::ControllerRequest::Start(spawn_workflow_request, reply_sender) => {
                                self.start_workflow(spawn_workflow_request, move |r| {
                                    tracing::debug!("Send Start Workflow Response");
                                    match reply_sender.send(r) {
                                        Ok(_) => {}
                                        Err(err) => {
                                            tracing::error!("Unhandled Send Reply Error: {err:?}");
                                        }
                                    }
                                }).await;

                            }
                            super::ControllerRequest::Stop(wf_id) => {
                                self.stop_workflow(&wf_id).await;
                            }
                            super::ControllerRequest::List(workflow_id, reply_sender) => {
                                let reply = self.list_workflows(&workflow_id).await;
                                match reply_sender.send(reply) {
                                    Ok(_) => {}
                                    Err(err) => {
                                        tracing::error!("Unhandled Send Reply Error: {err:?}");
                                    }
                                }
                            }
                            super::ControllerRequest::UpdateNode(update, reply_sender) => {
                                let reply = match update {
                                    edgeless_api::node_registration::UpdateNodeRequest::Registration(node_id, agent_url, invocation_url, resource_providers, capabilities, link_providers) => self.process_node_registration(node_id, agent_url, invocation_url, resource_providers, capabilities, link_providers).await,
                                    edgeless_api::node_registration::UpdateNodeRequest::Deregistration(node_id) => self.process_node_del(node_id).await,
                                };
                                match reply_sender.send(reply) {
                                    Ok(_) => {}
                                    Err(err) => {
                                        tracing::error!("Unhandled Send Reply Error: {err:?}");
                                    }
                                }
                            },
                            super::ControllerRequest::Patch(update) => {
                                let _res = self.patch_workflow(update).await;
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
        completion_hook: impl FnOnce(anyhow::Result<edgeless_api::workflow_instance::SpawnWorkflowResponse>) + Send + 'static,
    ) {
        // Assign a new identifier to the newly-created workflow.
        let wf_id = edgeless_api::workflow_instance::WorkflowId {
            workflow_id: uuid::Uuid::new_v4(),
        };

        let wf = super::super::ir::managed_worflow::ManagedWorkflow::new(
            spawn_workflow_request.clone(),
            wf_id.clone(),
            self.cluster_id.clone(),
            self.telemetry_provider.clone(),
            P::new(),
        );

        let cloned_nodes = self.nodes.clone();
        let cloned_state = self.global_pipeline_state.clone();
        let cloned_peers = self.peer_clusters.clone();

        self.active_workflows.insert(
            wf_id,
            super::workflow::WorkflowInstance::launch::<P>(
                wf,
                completion_hook,
                cloned_nodes,
                cloned_peers,
                cloned_state,
                self.image_repository.clone(),
            )
            .await,
        );
    }

    async fn stop_workflow(&mut self, wf_id: &edgeless_api::workflow_instance::WorkflowId) {
        let mut workflow = match self.active_workflows.remove(wf_id) {
            None => {
                tracing::info!("Trying to tear down a workflow that does not exist: {wf_id}");
                return;
            }
            Some(val) => val,
        };
        if let Err(e) = workflow.stop().await {
            tracing::error!("Could not stop workflow: {e:?}")
        }
    }

    async fn list_workflows(
        &mut self,
        workflow_id: &edgeless_api::workflow_instance::WorkflowId,
    ) -> anyhow::Result<Vec<edgeless_api::workflow_instance::WorkflowInstance>> {
        let ret: Vec<edgeless_api::workflow_instance::WorkflowInstance> = if let Some(_wf) = self.active_workflows.get(workflow_id) {
            vec![edgeless_api::workflow_instance::WorkflowInstance {
                workflow_id: workflow_id.clone(),
                //TODO(raphaelhetzel) Replace this with a new representation.
                node_mapping: Vec::new(),
            }]
        } else {
            self.active_workflows
                .keys()
                .map(|wf_id| edgeless_api::workflow_instance::WorkflowInstance {
                    workflow_id: wf_id.clone(),
                    //TODO(raphaelhetzel) Replace this with a new representation.
                    node_mapping: Vec::new(),
                })
                .collect()
        };
        Ok(ret)
    }

    async fn patch_workflow(&mut self, req: edgeless_api::common::PatchRequest) -> anyhow::Result<()> {
        let id = edgeless_api::workflow_instance::WorkflowId {
            workflow_id: req.function_id.function_id,
        };
        if let Some(wf) = self.active_workflows.get_mut(&id) {
            wf.patch(req).await;
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
        tracing::info!("Node Registration: {node_id}, {agent_url}, {invocation_url}");
        if let Some(node) = self.nodes.get(&node_id) {
            if node.agent_url() == agent_url && node.invocation_url() == invocation_url {
                return Ok(edgeless_api::node_registration::UpdateNodeResponse::Accepted);
            } else {
                return Ok(edgeless_api::node_registration::UpdateNodeResponse::ResponseError(ResponseError {
                    summary: "Duplicate NodeId with different URL(s).".to_string(),
                    detail: None,
                }));
            }
        }

        let n = super::node::WorkerNode::new(
            node_id,
            agent_url,
            invocation_url.clone(),
            resource_providers,
            capabilities,
            link_providers,
            self.telemetry_provider.clone(),
            self.cluster_id,
        )
        .await;

        self.nodes.insert(node_id, n.clone());

        self.send_peer_updates(vec![edgeless_api::node_management::UpdatePeersRequest::Add(node_id, invocation_url)])
            .await;

        // Send information about all nodes to the new node.
        let updates: Vec<_> = self
            .nodes
            .iter()
            .filter_map(|(n_id, n_spec)| {
                if n_id != &node_id {
                    Some(edgeless_api::node_management::UpdatePeersRequest::Add(*n_id, n_spec.invocation_url()))
                } else {
                    None
                }
            })
            .collect();
        {
            // let n = self.nodes.borrow_mut().get_mut(&node_id).unwrap();
            if let Some(node) = self.nodes.get_mut(&node_id) {
                node.update_peers(&updates).await.unwrap();
            }
        }

        for wf in self.active_workflows.values_mut() {
            wf.add_nodes(vec![n.clone()]).await;
        }

        Ok(edgeless_api::node_registration::UpdateNodeResponse::Accepted)
    }

    async fn process_node_del(
        &mut self,
        node_id: edgeless_api::function_instance::NodeId,
    ) -> anyhow::Result<edgeless_api::node_registration::UpdateNodeResponse> {
        let old_value = self.nodes.remove(&node_id);
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

    async fn periodic_health_check(&mut self) {
        // First check if there are nodes that must be disconnected
        // because they failed to reply to a keep-alive.
        let to_be_disconnected = self.find_dead_nodes().await;

        // Second, remove all those nodes from the map of clients.
        for node_id in to_be_disconnected.iter() {
            tracing::info!("Disconnected node not replying to keep-alive: {}", &node_id);
            let val = self.nodes.remove(node_id);
            assert!(val.is_some());
        }

        // Update the peers of (still alive) nodes by
        // deleting the missing-in-action peers.
        for removed_node_id in &to_be_disconnected {
            for (_, client_desc) in self.nodes.iter_mut() {
                match client_desc
                    .update_peers(&[edgeless_api::node_management::UpdatePeersRequest::Del(*removed_node_id)])
                    .await
                {
                    Ok(_) => {}
                    Err(err) => {
                        tracing::error!("Unhandled Update Peers Error: {err}");
                    }
                }
            }
        }
        if !to_be_disconnected.is_empty() {
            self.handle_node_removal(&to_be_disconnected).await;
        }
    }

    async fn find_dead_nodes(&mut self) -> std::collections::HashSet<edgeless_api::function_instance::NodeId> {
        let mut dead_nodes = std::collections::HashSet::new();
        for (node_id, client_desc) in self.nodes.iter_mut() {
            if client_desc.health_check().await.is_err() {
                dead_nodes.insert(*node_id);
            }
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
            if let Some(wf) = self.active_workflows.get_mut(&wf_id) {
                wf.remove_nodes(removed_nodes.clone()).await;
            }
        }
    }

    async fn send_peer_updates(&mut self, updates: Vec<edgeless_api::node_management::UpdatePeersRequest>) {
        for (_n_id, n_spec) in self.nodes.iter_mut() {
            n_spec.update_peers(&updates).await.unwrap();
        }
    }
}
