// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

pub struct ManagedWorkflow {
    pub wf: super::workflow::ActiveWorkflow,
    pub pipeline: super::transformations::TransformationPipeline,
}

impl ManagedWorkflow {
    pub fn new(
        request: edgeless_api::workflow_instance::SpawnWorkflowRequest,
        id: edgeless_api::workflow_instance::WorkflowId,
        orchestration_logic: std::sync::Arc<tokio::sync::Mutex<crate::orchestration_logic::OrchestrationLogic>>,
        nodes: std::sync::Arc<
            tokio::sync::Mutex<std::collections::HashMap<edgeless_api::function_instance::NodeId, crate::controller::server::WorkerNode>>,
        >,
        peer_clusters: std::sync::Arc<
            tokio::sync::Mutex<std::collections::HashMap<edgeless_api::function_instance::NodeId, crate::controller::server::PeerCluster>>,
        >,
        link_controllers: std::sync::Arc<
            tokio::sync::Mutex<std::collections::HashMap<edgeless_api::link::LinkType, Box<dyn edgeless_api::link::LinkController>>>,
        >,
    ) -> Self {
        Self {
            wf: super::workflow::ActiveWorkflow::new(request, id),
            pipeline: super::transformations::TransformationPipeline::new_default(orchestration_logic, nodes, peer_clusters, link_controllers),
        }
    }

    pub fn initial_spawn(&mut self) -> Vec<super::RequiredChange> {
        self.pipeline.apply_all(&mut self.wf);
        self.materialize()
    }

    pub fn node_removal(
        &mut self,
        removed_node_ids: &std::collections::HashSet<edgeless_api::function_instance::NodeId>,
    ) -> Vec<super::RequiredChange> {
        if self.remove_nodes(removed_node_ids) {
            self.pipeline.apply_all(&mut self.wf);
            self.materialize()
        } else {
            Vec::new()
        }
    }

    pub fn patch_external_links(&mut self, update: edgeless_api::common::PatchRequest) -> Vec<super::RequiredChange> {
        {
            let mut prx = self.wf.proxy.borrow_mut();
            prx.external_ports.external_input_mapping = update.input_mapping;
            prx.external_ports.external_output_mapping = update.output_mapping;
        }
        self.pipeline.apply_all(&mut self.wf);
        self.materialize()
    }

    pub fn peer_cluster_removal(&self, removed_cluster_ids: edgeless_api::function_instance::NodeId) -> Vec<super::RequiredChange> {
        Vec::new()
    }

    pub fn stop(&mut self) -> Vec<super::RequiredChange> {
        // TODO
        Vec::new()
    }

    fn materialize(&mut self) -> Vec<super::RequiredChange> {
        let mut changes = Vec::new();

        for (link_id, link) in &mut self.wf.links {
            if !link.materialized {
                changes.push(super::RequiredChange::InstantiateLinkControlPlane {
                    link_id: link_id.clone(),
                    class: link.class.clone(),
                });
            }

            for (node, link_provider_id, node_config, node_materialized) in &link.nodes {
                if !node_materialized {
                    changes.push(super::RequiredChange::CreateLinkOnNode {
                        node_id: *node,
                        provider_id: link_provider_id.clone(),
                        link_id: link_id.clone(),
                        config: node_config.clone(),
                    });
                }
            }
        }

        for (f_name, function) in &self.wf.functions {
            let function = function.borrow_mut();
            for i in function.instances.iter() {
                let mut current = i.borrow_mut();
                if let Some(materialized) = &current.materialized {
                    if materialized.physical_input_mapping != current.desired_mapping.physical_input_mapping
                        || materialized.physical_output_mapping != current.desired_mapping.physical_output_mapping
                    {
                        changes.push(super::RequiredChange::PatchFunction {
                            function_id: current.id,
                            function_name: f_name.clone(),
                            input_mapping: current.desired_mapping.physical_input_mapping.clone(),
                            output_mapping: current.desired_mapping.physical_output_mapping.clone(),
                        });
                    }
                } else {
                    changes.push(super::RequiredChange::StartFunction {
                        function_id: current.id,
                        function_name: f_name.clone(),
                        image: if let Some(custom_image) = &current.image {
                            custom_image.clone()
                        } else {
                            function.image.clone()
                        },
                        input_mapping: current.desired_mapping.physical_input_mapping.clone(),
                        output_mapping: current.desired_mapping.physical_output_mapping.clone(),
                        annotations: function.annotations.clone(),
                    });
                    current.materialized = Some(super::PhysicalPorts {
                        physical_input_mapping: current.desired_mapping.physical_input_mapping.clone(),
                        physical_output_mapping: current.desired_mapping.physical_output_mapping.clone(),
                    });
                }
            }
        }

        for (r_name, resource) in &mut self.wf.resources {
            let resource = resource.borrow_mut();
            for i in &resource.instances {
                let mut current = i.borrow_mut();
                if let Some(materialized) = &current.materialized {
                    if materialized.physical_input_mapping != current.desired_mapping.physical_input_mapping
                        || materialized.physical_output_mapping != current.desired_mapping.physical_output_mapping
                    {
                        changes.push(super::RequiredChange::PatchResource {
                            resource_id: current.id,
                            resource_name: r_name.clone(),
                            input_mapping: current.desired_mapping.physical_input_mapping.clone(),
                            output_mapping: current.desired_mapping.physical_output_mapping.clone(),
                        });
                    }
                } else {
                    changes.push(super::RequiredChange::StartResource {
                        resource_id: current.id,
                        resource_name: r_name.clone(),
                        class_type: resource.class.clone(),
                        input_mapping: current.desired_mapping.physical_input_mapping.clone(),
                        output_mapping: current.desired_mapping.physical_output_mapping.clone(),
                        configuration: resource.configurations.clone(),
                    });
                    current.materialized = Some(super::PhysicalPorts {
                        physical_input_mapping: current.desired_mapping.physical_input_mapping.clone(),
                        physical_output_mapping: current.desired_mapping.physical_output_mapping.clone(),
                    });
                }
            }
        }

        for (_s_name, subflow) in &mut self.wf.subflows {
            let subflow = subflow.borrow_mut();
            for i in &subflow.instances {
                let current = i.borrow_mut();
                if let Some(materialized) = &current.materialized {
                    if materialized.physical_input_mapping != current.desired_mapping.physical_input_mapping
                        || materialized.physical_output_mapping != current.desired_mapping.physical_output_mapping
                    {
                        changes.push(super::RequiredChange::PatchSubflow {
                            subflow_id: current.id,
                            input_mapping: current.desired_mapping.physical_input_mapping.clone(),
                            output_mapping: current.desired_mapping.physical_output_mapping.clone(),
                        });
                    }
                } else {
                    changes.push(super::RequiredChange::CreateSubflow {
                        subflow_id: current.id,
                        spawn_req: edgeless_api::workflow_instance::SpawnWorkflowRequest {
                            workflow_functions: Vec::new(),
                            workflow_resources: Vec::new(),
                            workflow_ingress_proxies: current
                                .desired_mapping
                                .physical_input_mapping
                                .iter()
                                .map(|(id, physical_port)| edgeless_api::workflow_instance::WorkflowIngressProxy {
                                    id: id.0.clone(),
                                    inner_output: subflow.logical_ports.logical_output_mapping.get(id).unwrap().clone(),
                                    external_input: physical_port.clone(),
                                })
                                .collect(),
                            workflow_egress_proxies: current
                                .desired_mapping
                                .physical_output_mapping
                                .iter()
                                .map(|(id, physical_port)| edgeless_api::workflow_instance::WorkflowEgressProxy {
                                    id: id.0.clone(),
                                    inner_input: match subflow.logical_ports.logical_input_mapping.get(id).unwrap().clone() {
                                        super::LogicalInput::Direct(vec) => edgeless_api::workflow_instance::PortMapping::AnyOfTargets(vec),
                                        super::LogicalInput::Topic(topic) => edgeless_api::workflow_instance::PortMapping::Topic(topic),
                                    },
                                    external_output: physical_port.clone(),
                                })
                                .collect(),
                            annotations: std::collections::HashMap::new(),
                        },
                    });
                }
            }
        }

        {
            let prx = self.wf.proxy.borrow_mut();
            for i in &prx.instances {
                let current = i.borrow_mut();
                if let Some(materialized) = &current.materialized {
                    if materialized.physical_input_mapping != current.desired_mapping.physical_input_mapping
                        || materialized.physical_output_mapping != current.desired_mapping.physical_output_mapping
                    {
                        changes.push(super::RequiredChange::PatchProxy {
                            proxy_id: current.id,
                            internal_inputs: current.desired_mapping.physical_input_mapping.clone(),
                            internal_outputs: current.desired_mapping.physical_output_mapping.clone(),
                            external_inputs: prx.external_ports.external_input_mapping.clone(),
                            external_outputs: prx.external_ports.external_output_mapping.clone(),
                        })
                    } else {
                        changes.push(super::RequiredChange::CrateProxy {
                            proxy_id: current.id,
                            internal_inputs: current.desired_mapping.physical_input_mapping.clone(),
                            internal_outputs: current.desired_mapping.physical_output_mapping.clone(),
                            external_inputs: prx.external_ports.external_input_mapping.clone(),
                            external_outputs: prx.external_ports.external_output_mapping.clone(),
                        });
                    }
                }
            }
        }

        changes
    }

    fn remove_nodes(&mut self, node_ids: &std::collections::HashSet<edgeless_api::function_instance::NodeId>) -> bool {
        let mut changed = false;
        for (_, function) in &mut self.wf.functions {
            let mut function = function.borrow_mut();
            let before = function.instances.len();
            function
                .instances
                .retain(|instance| !node_ids.contains(&instance.borrow_mut().id.node_id));
            if before != function.instances.len() {
                changed = true;
            }
        }
        for (_, resource) in &mut self.wf.resources {
            let mut resource = resource.borrow_mut();
            let before = resource.instances.len();
            resource
                .instances
                .retain(|instance| !node_ids.contains(&instance.borrow_mut().id.node_id));
            if before != resource.instances.len() {
                changed = true;
            }
        }
        changed
    }
}
