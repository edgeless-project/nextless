// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-FileCopyrightText: © 2023 University of Cambridge, System Research Group
// SPDX-FileCopyrightText: © 2024 Roman Kolcun <roman.kolcun@cl.cam.ac.uk>
// SPDX-License-Identifier: MIT

use crate::ir::{Node, RequiredChange};
use edgeless_api::image_repository::FunctionImageHash;
use std::io::Write;

pub struct WorkflowInstance {
    inner: WorkflowInstanceState,
}

enum WorkflowInstanceState {
    Active {
        task: tokio::task::JoinHandle<Result<(), WorkflowError>>,
        sender: tokio::sync::mpsc::UnboundedSender<WorkflowManagementEvent>,
    },
    Stopped,
}

struct WorkflowTask<P: crate::ir::transformations::placement::strategy::PlacementStrategy> {
    wf: crate::ir::managed_worflow::ManagedWorkflow<P>,
    global_pipeline_state: std::sync::Arc<crate::ir::pipeline::default::DefaultTransformationPipelineState<P::GlobalState>>,
    receiver: tokio::sync::mpsc::UnboundedReceiver<WorkflowManagementEvent>,
    nodes: std::collections::HashMap<edgeless_api::function_instance::NodeId, super::node::WorkerNode>,
    peer_clusters: std::collections::HashMap<edgeless_api::function_instance::NodeId, super::peer_cluster::PeerCluster>,
    image_repository: super::image_repository::ImageRepository,
    duration_log_file: Option<std::fs::File>,
}

enum WorkflowManagementEvent {
    PatchExternal(edgeless_api::common::PatchRequest),
    Stop,
    NodesAdded(Vec<super::node::WorkerNode>),
    NodesRemoved(std::collections::HashSet<edgeless_api::function_instance::NodeId>),
}

#[derive(thiserror::Error, Debug)]
pub enum WorkflowError {
    #[error("WorkflowTask was stopped.")]
    InputClosed,
    #[error("Could not join WorkflowTask.")]
    TaskJoinFailed,
    #[error("Interacted with stopped Workflow.")]
    WrongState,
}

pub type WorkflowResult = Result<(), WorkflowError>;

impl WorkflowInstance {
    pub(crate) async fn launch<P: crate::ir::transformations::placement::strategy::PlacementStrategy + 'static>(
        wf: super::super::ir::managed_worflow::ManagedWorkflow<P>,
        start_completed: impl FnOnce(anyhow::Result<edgeless_api::workflow_instance::SpawnWorkflowResponse>) + Send + 'static,
        initial_nodes: std::collections::HashMap<edgeless_api::function_instance::NodeId, super::node::WorkerNode>,
        initial_peer_clusters: std::collections::HashMap<edgeless_api::function_instance::NodeId, super::peer_cluster::PeerCluster>,
        global_pipeline_state: std::sync::Arc<crate::ir::pipeline::default::DefaultTransformationPipelineState<P::GlobalState>>,
        image_repository: super::image_repository::ImageRepository,
    ) -> Self {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();

        let duration_log_file = if let Ok(filename) = std::env::var("EDGELESS_DEBUG_FILE") {
            let file = std::fs::OpenOptions::new().write(true).append(true).create(true).open(filename).unwrap();
            Some(file)
        } else {
            None
        };

        let t = WorkflowTask {
            wf,
            global_pipeline_state,
            receiver,
            nodes: initial_nodes,
            peer_clusters: initial_peer_clusters,
            image_repository,
            duration_log_file: duration_log_file,
        };

        let task = tokio::task::spawn(WorkflowTask::run(t, start_completed));

        Self {
            inner: WorkflowInstanceState::Active { task, sender },
        }
    }

    pub(crate) async fn patch(&mut self, req: edgeless_api::common::PatchRequest) {
        if let WorkflowInstanceState::Active { sender, .. } = &mut self.inner {
            sender.send(WorkflowManagementEvent::PatchExternal(req)).unwrap_or_else(|_| {
                tracing::warn!("Tried to patch dead workflow instance.");
                self.inner = WorkflowInstanceState::Stopped;
            });
        }
    }

    pub(crate) async fn remove_nodes(&mut self, removed_nodes: std::collections::HashSet<edgeless_api::function_instance::NodeId>) {
        if let WorkflowInstanceState::Active { sender, .. } = &mut self.inner {
            sender
                .send(WorkflowManagementEvent::NodesRemoved(removed_nodes))
                .unwrap_or_else(|_| tracing::warn!("Tried to remove node from dead workflow instance."));
        }
    }

    pub(crate) async fn add_nodes(&mut self, added_nodes: Vec<super::node::WorkerNode>) {
        if let WorkflowInstanceState::Active { sender, .. } = &mut self.inner {
            sender
                .send(WorkflowManagementEvent::NodesAdded(added_nodes))
                .unwrap_or_else(|_| tracing::warn!("Tried to add node to dead workflow instance."));
        }
    }

    pub(crate) async fn stop(&mut self) -> WorkflowResult {
        // https://stackoverflow.com/questions/36557412/how-can-i-change-enum-variant-while-moving-the-field-to-the-new-variant
        if let WorkflowInstanceState::Active { sender, task } = std::mem::replace(&mut self.inner, WorkflowInstanceState::Stopped) {
            sender
                .send(WorkflowManagementEvent::Stop)
                .unwrap_or_else(|_| tracing::warn!("Tried to stop dead workflow instance."));

            task.await.map_err(|_| WorkflowError::TaskJoinFailed)?
        } else {
            Err(WorkflowError::WrongState)
        }
    }
}

impl<P: crate::ir::transformations::placement::strategy::PlacementStrategy + 'static> WorkflowTask<P> {
    async fn run(
        mut self,
        on_start_completed: impl FnOnce(anyhow::Result<edgeless_api::workflow_instance::SpawnWorkflowResponse>) + Send,
    ) -> WorkflowResult {
        self.launch(on_start_completed).await?;

        loop {
            match tokio::time::timeout(tokio::time::Duration::from_secs(1), self.receiver.recv()).await {
                Ok(management_event) => {
                    let event = management_event.ok_or(WorkflowError::InputClosed)?;
                    let is_stop = matches!(&event, WorkflowManagementEvent::Stop);
                    self.handle_management_event(event).await?;
                    if is_stop {
                        return Ok(());
                    }
                }
                Err(_timeout) => self.optimize().await?,
            };
        }
    }

    #[tracing::instrument(name = "controller_management_event", skip_all, fields(workflow_id = self.wf.wf.id.to_string()))]
    async fn handle_management_event(&mut self, event: WorkflowManagementEvent) -> Result<(), WorkflowError> {
        match event {
            WorkflowManagementEvent::PatchExternal(patch_request) => self.patch(&patch_request).await,
            WorkflowManagementEvent::Stop => self.stop().await,
            WorkflowManagementEvent::NodesAdded(added_nodes) => self.node_addition(added_nodes).await,
            WorkflowManagementEvent::NodesRemoved(removed_nodes) => self.node_removal(&removed_nodes).await,
        }
    }

    #[tracing::instrument(name = "controller_workflow_launch", skip_all, fields(workflow_id = self.wf.wf.id.to_string()))]
    async fn launch(
        &mut self,
        on_start_completed: impl FnOnce(anyhow::Result<edgeless_api::workflow_instance::SpawnWorkflowResponse>) + Send,
    ) -> WorkflowResult {
        let start = tokio::time::Instant::now();

        let required_changes = {
            tokio::task::block_in_place(|| {
                let ir_nodes: std::collections::HashMap<edgeless_api::function_instance::NodeId, &dyn crate::ir::Node> =
                    self.nodes.iter().map(|(n_id, node)| (*n_id, node as &dyn crate::ir::Node)).collect();
                self.wf
                    .initial_spawn(&ir_nodes, &std::collections::HashMap::new(), &self.global_pipeline_state)
            })
        };

        let desc = edgeless_api::workflow_instance::WorkflowInstance {
            workflow_id: self.wf.wf.id.clone(),
            node_mapping: self
                .wf
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

        // self.active_workflows.insert(wf_id.clone(), wf);

        let res = self.materialize(required_changes).await;

        if let Err(errors) = &res {
            tracing::warn!("Failure starting workflow: {}", errors.join("\n"));
            self.stop().await?;
        }

        on_start_completed(match res {
            Ok(_) => Ok(edgeless_api::workflow_instance::SpawnWorkflowResponse::WorkflowInstance(desc)),
            Err(err) => Ok(edgeless_api::workflow_instance::SpawnWorkflowResponse::ResponseError(
                edgeless_api::common::ResponseError {
                    summary: "Workflow creation failed".to_string(),
                    detail: Some(err.join(";")),
                },
            )),
        });

        let duration = start.elapsed();
        if let Some(duration_log_file) = &mut self.duration_log_file {
            duration_log_file
                .write_fmt(format_args!("initial_spawn,{}\n", duration.as_micros()))
                .unwrap();
        }

        Ok(())
    }

    async fn patch(&mut self, req: &edgeless_api::common::PatchRequest) -> WorkflowResult {
        let required_changes = {
            let ir_nodes: std::collections::HashMap<edgeless_api::function_instance::NodeId, &dyn crate::ir::Node> =
                self.nodes.iter().map(|(n_id, node)| (*n_id, node as &dyn crate::ir::Node)).collect();
            tokio::task::block_in_place(|| {
                self.wf
                    .patch_external_links(req.clone(), &ir_nodes, &std::collections::HashMap::new(), &self.global_pipeline_state)
            })
        };
        if let Err(errs) = self.materialize(required_changes).await {
            tracing::info!("Failures materializing workflow patch: {}.", errs.join(";"));
        };
        Ok(())
    }

    async fn node_removal(&mut self, removed_nodes: &std::collections::HashSet<edgeless_api::function_instance::NodeId>) -> WorkflowResult {
        let start = tokio::time::Instant::now();

        for removed_node in removed_nodes {
            self.nodes.remove(removed_node);
        }

        let required_changes = {
            let ir_nodes: std::collections::HashMap<edgeless_api::function_instance::NodeId, &dyn crate::ir::Node> =
                self.nodes.iter().map(|(n_id, node)| (*n_id, node as &dyn crate::ir::Node)).collect();
            tokio::task::block_in_place(|| {
                self.wf
                    .node_removal(removed_nodes, &ir_nodes, &std::collections::HashMap::new(), &self.global_pipeline_state)
            })
        };
        if let Err(errs) = self.materialize(required_changes).await {
            tracing::error!("Failures materializing node removal: {}.", errs.join(";"));
        }

        let duration = start.elapsed();
        if let Some(duration_log_file) = &mut self.duration_log_file {
            duration_log_file
                .write_fmt(format_args!("node_removal,{}\n", duration.as_micros()))
                .unwrap();
        }

        Ok(())
    }

    async fn node_addition(&mut self, added_nodes: Vec<super::node::WorkerNode>) -> WorkflowResult {
        for n in added_nodes {
            self.nodes.insert(n.node_id(), n);
        }
        self.optimize().await
    }

    async fn stop(&mut self) -> WorkflowResult {
        let changes = self.wf.stop();
        if let Err(errs) = self.materialize(changes).await {
            tracing::info!("Failures materializing workflow stop: {}.", errs.join(";"));
        };
        Ok(())
    }

    #[tracing::instrument(name = "controller_workflow_optimize", skip_all, fields(workflow_id = self.wf.wf.id.to_string()))]
    async fn optimize(&mut self) -> WorkflowResult {
        let required_changes = {
            let ir_nodes: std::collections::HashMap<edgeless_api::function_instance::NodeId, &dyn crate::ir::Node> =
                self.nodes.iter().map(|(n_id, node)| (*n_id, node as &dyn crate::ir::Node)).collect();
            tokio::task::block_in_place(|| {
                self.wf
                    .periodic_optimize(&ir_nodes, &std::collections::HashMap::new(), &self.global_pipeline_state)
            })
        };
        if let Err(errs) = self.materialize(required_changes).await {
            tracing::error!("Failures materializing periodic optimization: {}.", errs.join(";"));
        }
        // TODO: Decide on how to handle these errors
        Ok(())
    }

    #[tracing::instrument(name = "materialize", skip_all, fields(workflow_id = self.wf.wf.id.to_string()))]
    async fn materialize(
        &mut self,
        // wf_id: edgeless_api::workflow_instance::WorkflowId,
        required_changes: Vec<RequiredChange>,
    ) -> Result<(), Vec<String>> {
        let mut results = Vec::<Result<(), String>>::new();

        let wf_id = self.wf.wf.id.clone();

        // This could be parallel
        for f in required_changes.into_iter() {
            results.push(match f {
                RequiredChange::StartFunction {
                    function_id,
                    image,
                    behavior_spec,
                    input_mapping,
                    output_mapping,
                    function_name,
                    annotations,
                } => {
                    self.start_workflow_function_on_node(
                        &wf_id,
                        function_name,
                        function_id,
                        image,
                        behavior_spec,
                        input_mapping,
                        output_mapping,
                        annotations,
                    )
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
                RequiredChange::StopFunction { function_id } => self.stop_workflow_function_on_node(function_id).await,
                RequiredChange::StopResource { resource_id } => self.stop_workflow_resource_on_node(resource_id).await,
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

    #[allow(clippy::too_many_arguments)]
    async fn start_workflow_function_on_node(
        &mut self,
        wf_id: &edgeless_api::workflow_instance::WorkflowId,
        f_name: String,
        function_id: edgeless_api::function_instance::InstanceId,
        image: super::super::ir::behavior::BehaviorImage,
        behavior_spec: super::super::ir::behavior::BehaviorSpec,
        input_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, super::super::ir::PhysicalInput>,
        output_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, super::super::ir::PhysicalOutput>,
        annotations: std::collections::HashMap<String, String>,
    ) -> Result<(), String> {
        // [TODO] Issue#95
        // The state_specification configuration should be
        // read from the function annotations.
        tracing::debug!("State specifications currently forced to NodeLocal.");

        self.image_repository.update(image.image.image_hash(), image.image.clone()).await;

        let (api_output_mapping, api_input_mapping) = Self::convert_mappings(output_mapping, input_mapping);

        let response = self
            .fn_client(&function_id.node_id)
            .await
            .ok_or(format!("No function client for node: {}", &function_id.node_id))?
            .start(edgeless_api::function_instance::SpawnFunctionRequest {
                instance_id: function_id,
                code: edgeless_api::function_instance::FunctionClassSpecification {
                    function_class_id: image.behavior_image_id.behavior_id.id.clone(),
                    function_class_type: image.behavior_image_id.dialect_type.base_type.to_string(),
                    function_class_version: image.behavior_image_id.behavior_id.version.clone(),
                    function_class_code: image.image.clone(),
                    function_class_outputs: behavior_spec.output_ports.into_iter().collect(),
                    function_class_inputs: behavior_spec.input_ports.into_iter().collect(),
                    function_class_inner_structure: behavior_spec
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
                input_mapping: api_input_mapping,
                output_mapping: api_output_mapping,
            })
            .await;

        match response {
            Ok(response) => match response {
                edgeless_api::common::StartComponentResponse::ResponseError(error) => {
                    tracing::warn!("Function instance {wf_id}:{f_name} creation rejected: {error}");
                    Err(format!("function instance creation rejected: {error} "))
                }
                edgeless_api::common::StartComponentResponse::InstanceId(id) => {
                    tracing::info!("Workflow {} function {} started with fid {}", wf_id, &f_name, &id);
                    Ok(())
                }
            },
            Err(err) => Err(format!("failed interaction when creating a function instance: {err}")),
        }
    }

    async fn stop_workflow_function_on_node(&mut self, function_id: edgeless_api::function_instance::InstanceId) -> Result<(), String> {
        if let Some(node_api) = self.nodes.get_mut(&function_id.node_id) {
            if let Err(e) = node_api.fn_client().unwrap().stop(function_id).await {
                Err(format!("Stopping Node Function Failed: {e}"))
            } else {
                Ok(())
            }
        } else {
            Err("Invalid Function ID".to_string())
        }
    }

    #[allow(clippy::too_many_arguments)]
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
        let (api_output_mapping, api_input_mapping) = Self::convert_mappings(output_mapping, input_mapping);

        let response = self
            .resource_client(&resource_id.node_id)
            .await
            .ok_or(format!("No resource client for node: {}", &resource_id.node_id))?
            .start(edgeless_api::resource_configuration::ResourceInstanceSpecification {
                resource_id,
                class_type: class_type.clone(),
                configuration: configurations.clone(),
                output_mapping: api_output_mapping,
                input_mapping: api_input_mapping,
            })
            .await;

        match response {
            Ok(response) => match response {
                edgeless_api::common::StartComponentResponse::ResponseError(error) => {
                    tracing::warn!("Resource start rejected: {error}");
                    Err(format!("resource start rejected: {error} "))
                }
                edgeless_api::common::StartComponentResponse::InstanceId(id) => {
                    tracing::info!("Workflow {} resource {} started with fid {}", wf_id, &r_name, &id);
                    Ok(())
                }
            },
            Err(err) => Err(format!("failed interaction when starting a resource: {err}")),
        }
    }

    async fn stop_workflow_resource_on_node(&mut self, resource_id: edgeless_api::function_instance::InstanceId) -> Result<(), String> {
        if let Some(node_api) = self.nodes.get_mut(&resource_id.node_id) {
            if let Err(e) = node_api.resource_client().unwrap().stop(resource_id).await {
                Err(format!("Stopping Node Resource Failed: {e}"))
            } else {
                Ok(())
            }
        } else {
            Err("Invalid Resource ID".to_string())
        }
    }

    async fn start_subflow_on_cluster(
        &mut self,
        subflow_id: edgeless_api::function_instance::InstanceId,
        spawn_req: edgeless_api::workflow_instance::SpawnWorkflowRequest,
    ) -> Result<(), String> {
        if let Some(mut cluster_api) = self.workflow_client(&subflow_id.node_id).await {
            cluster_api.start(spawn_req).await.map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Err("Failed to start subflow".to_string())
        }
    }

    async fn start_proxy_on_node(
        &mut self,
        proxy_id: edgeless_api::function_instance::InstanceId,
        internal_inputs: std::collections::HashMap<edgeless_api::function_instance::PortId, crate::ir::interaction::DestiantionPortMapping>,
        internal_outputs: std::collections::HashMap<edgeless_api::function_instance::PortId, crate::ir::interaction::SourcePortMapping>,
        external_inputs: std::collections::HashMap<edgeless_api::function_instance::PortId, crate::ir::interaction::DestiantionPortMapping>,
        external_outputs: std::collections::HashMap<edgeless_api::function_instance::PortId, crate::ir::interaction::SourcePortMapping>,
    ) -> Result<(), String> {
        let (internal_api_outputs, internal_api_inputs) = Self::convert_mappings(internal_outputs, internal_inputs);
        let (external_api_outputs, external_api_inputs) = Self::convert_mappings(external_outputs, external_inputs);

        match self
            .proxy_client(&proxy_id.node_id)
            .await
            .ok_or(format!("No proxy client for node {}", proxy_id.node_id))?
            .start(edgeless_api::proxy_instance::ProxySpec {
                instance_id: proxy_id,
                inner_outputs: internal_api_outputs,
                inner_inputs: internal_api_inputs,
                external_outputs: external_api_outputs,
                external_inputs: external_api_inputs,
            })
            .await
        {
            Ok(_) => Ok(()),
            Err(err) => Err(format!("failed starting proxy: {err}")),
        }
    }

    async fn create_link_on_node(
        &mut self,
        link_id: edgeless_api::link::LinkInstanceId,
        node_id: edgeless_api::function_instance::NodeId,
        link_provider_id: edgeless_api::link::LinkProviderId,
        config: Vec<u8>,
    ) -> Result<(), String> {
        if let Some(node) = self.nodes.get_mut(&node_id) {
            node.link_instance_client()
                .unwrap()
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
        if let Some(node) = self.nodes.get_mut(&node_id) {
            node.link_instance_client().unwrap().remove(link_id).await.map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Err("Node Not Found".to_string())
        }
    }

    async fn patch_outputs(
        &mut self,
        origin_id: edgeless_api::function_instance::InstanceId,
        origin_type: super::ComponentType,
        output_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, crate::ir::interaction::SourcePortMapping>,
        input_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, crate::ir::interaction::DestiantionPortMapping>,
        name_in_workflow: &str,
    ) -> Result<(), String> {
        let (api_output_mapping, api_input_mapping) = Self::convert_mappings(output_mapping, input_mapping);

        match origin_type {
            super::ComponentType::Function => {
                match self
                    .fn_client(&origin_id.node_id)
                    .await
                    .ok_or(format!("No function client for node: {}", origin_id.node_id))?
                    .patch(edgeless_api::common::PatchRequest {
                        function_id: origin_id,
                        output_mapping: api_output_mapping,
                        input_mapping: api_input_mapping,
                    })
                    .await
                {
                    Ok(_) => Ok(()),
                    Err(err) => Err(format!("failed interaction when patching component {name_in_workflow}: {err}")),
                }
            }
            super::ComponentType::Resource => {
                match self
                    .resource_client(&origin_id.node_id)
                    .await
                    .ok_or(format!("No resource client for node: {}", origin_id.node_id))?
                    .patch(edgeless_api::common::PatchRequest {
                        function_id: origin_id,
                        output_mapping: api_output_mapping,
                        input_mapping: api_input_mapping,
                    })
                    .await
                {
                    Ok(_) => Ok(()),
                    Err(err) => Err(format!("failed interaction when patching component {name_in_workflow}: {err}")),
                }
            }
            super::ComponentType::SubFlow => {
                match self
                    .workflow_client(&origin_id.node_id)
                    .await
                    .ok_or(format!("No workflow client for cluster {}", origin_id.node_id))?
                    .patch(edgeless_api::common::PatchRequest {
                        function_id: origin_id,
                        output_mapping: api_output_mapping,
                        input_mapping: api_input_mapping,
                    })
                    .await
                {
                    Ok(_) => Ok(()),
                    Err(err) => Err(format!("failed interaction when patching component {name_in_workflow}: {err}")),
                }
            }
        }
    }

    fn convert_mappings(
        output_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, crate::ir::interaction::SourcePortMapping>,
        input_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, crate::ir::interaction::DestiantionPortMapping>,
    ) -> (
        std::collections::HashMap<edgeless_api::function_instance::PortId, edgeless_api::common::Output>,
        std::collections::HashMap<edgeless_api::function_instance::PortId, edgeless_api::common::Input>,
    ) {
        let mut api_output_mapping = std::collections::HashMap::<edgeless_api::function_instance::PortId, edgeless_api::common::Output>::new();
        let mut api_input_mapping = std::collections::HashMap::<edgeless_api::function_instance::PortId, edgeless_api::common::Input>::new();

        for (port, output_port) in output_mapping {
            let any_mapping = output_port.mapping.as_ref() as &dyn std::any::Any;

            let maybe_overlay_mapping = any_mapping.downcast_ref::<crate::ir::interaction::dialect::physical_overlay::PhysicalOverlaySourcePort>();
            if let Some(overlay_mapping) = maybe_overlay_mapping {
                match &overlay_mapping.destination {
                    crate::ir::interaction::dialect::physical_overlay::DestinationMapping::Unicast(physical_port_id) => {
                        api_output_mapping.insert(
                            port,
                            edgeless_api::common::Output::Single(physical_port_id.instance, physical_port_id.port.clone()),
                        );
                    }
                    crate::ir::interaction::dialect::physical_overlay::DestinationMapping::Anycast(physical_port_ids) => {
                        api_output_mapping.insert(
                            port,
                            edgeless_api::common::Output::Any(physical_port_ids.iter().map(|p| (p.instance, p.port.clone())).collect()),
                        );
                    }
                    crate::ir::interaction::dialect::physical_overlay::DestinationMapping::Multicast(physical_port_ids) => {
                        api_output_mapping.insert(
                            port,
                            edgeless_api::common::Output::Any(physical_port_ids.iter().map(|p| (p.instance, p.port.clone())).collect()),
                        );
                    }
                }
                continue;
            }

            let maybe_multicast_mapping = any_mapping.downcast_ref::<crate::ir::interaction::dialect::ip_multicast::IpMulticastSourcePort>();
            if let Some(overlay_mapping) = maybe_multicast_mapping {
                api_output_mapping.insert(port, edgeless_api::common::Output::Link(overlay_mapping.link_id.clone()));
                continue;
            }

            tracing::warn!("Output mapping uses unsupported dialect: {:?}!", output_port.dialect_type.base_type);
        }

        for (port, output_port) in input_mapping {
            let any_mapping = output_port.mapping.as_ref() as &dyn std::any::Any;

            let maybe_overlay_mapping =
                any_mapping.downcast_ref::<crate::ir::interaction::dialect::physical_overlay::PhysicalOverlayDestinationPort>();
            if let Some(_overlay_mapping) = maybe_overlay_mapping {
                api_input_mapping.insert(port, edgeless_api::common::Input::Stub);
                continue;
            }

            let maybe_overlay_mapping = any_mapping.downcast_ref::<crate::ir::interaction::dialect::ip_multicast::IpMulticastDestinationPort>();
            if let Some(overlay_mapping) = maybe_overlay_mapping {
                api_input_mapping.insert(port, edgeless_api::common::Input::Link(overlay_mapping.link_id.clone()));
                continue;
            }

            tracing::warn!("Input mapping uses unsupported dialect: {:?}!", output_port.dialect_type.base_type);
        }

        (api_output_mapping, api_input_mapping)
    }

    async fn patch_proxy_instance(
        &mut self,
        proxy_id: edgeless_api::function_instance::InstanceId,
        internal_inputs: std::collections::HashMap<edgeless_api::function_instance::PortId, crate::ir::interaction::DestiantionPortMapping>,
        internal_outputs: std::collections::HashMap<edgeless_api::function_instance::PortId, crate::ir::interaction::SourcePortMapping>,
        external_inputs: std::collections::HashMap<edgeless_api::function_instance::PortId, crate::ir::interaction::DestiantionPortMapping>,
        external_outputs: std::collections::HashMap<edgeless_api::function_instance::PortId, crate::ir::interaction::SourcePortMapping>,
    ) -> Result<(), String> {
        let (internal_api_outputs, internal_api_inputs) = Self::convert_mappings(internal_outputs, internal_inputs);
        let (external_api_outputs, external_api_inputs) = Self::convert_mappings(external_outputs, external_inputs);

        match self
            .proxy_client(&proxy_id.node_id)
            .await
            .ok_or(format!("No proxy client for node {}", proxy_id.node_id))?
            .patch(edgeless_api::proxy_instance::ProxySpec {
                instance_id: proxy_id,
                inner_outputs: internal_api_outputs,
                inner_inputs: internal_api_inputs,
                external_outputs: external_api_outputs,
                external_inputs: external_api_inputs,
            })
            .await
        {
            Ok(_) => Ok(()),
            Err(err) => Err(format!("failed patching proxy {err}")),
        }
    }

    async fn create_link_control_plane(
        &mut self,
        link_id: edgeless_api::link::LinkInstanceId,
        class: edgeless_api::link::LinkType,
    ) -> Result<(), String> {
        self.global_pipeline_state
            .interaction_dialect_registry
            .lock()
            .await
            .instantiate_link_data_plane(link_id, class)
            .await
    }

    async fn fn_client(
        &mut self,
        node_id: &edgeless_api::function_instance::NodeId,
    ) -> Option<Box<dyn edgeless_api::function_instance::FunctionInstanceAPI<edgeless_api::function_instance::InstanceId>>> {
        self.nodes.get_mut(node_id)?.fn_client()
    }

    async fn resource_client(
        &mut self,
        node_id: &edgeless_api::function_instance::NodeId,
    ) -> Option<Box<dyn edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::InstanceId>>> {
        self.nodes.get_mut(node_id)?.resource_client()
    }

    async fn workflow_client(
        &mut self,
        cluster_id: &edgeless_api::function_instance::NodeId,
    ) -> Option<Box<dyn edgeless_api::workflow_instance::WorkflowInstanceAPI>> {
        Some(self.peer_clusters.get_mut(cluster_id)?.api.workflow_instance_api())
    }

    async fn proxy_client(
        &mut self,
        node_id: &edgeless_api::function_instance::NodeId,
    ) -> Option<Box<dyn edgeless_api::proxy_instance::ProxyInstanceAPI>> {
        self.nodes.get_mut(node_id)?.proxy_client()
    }
}
