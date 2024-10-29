// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

use super::super::*;

pub struct DefaultPlacement {
    orchestration_logic: std::sync::Arc<tokio::sync::Mutex<crate::orchestration_logic::OrchestrationLogic>>,
    nodes:
        std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<edgeless_api::function_instance::NodeId, crate::controller::server::WorkerNode>>>,
    peer_clusters: std::sync::Arc<
        tokio::sync::Mutex<std::collections::HashMap<edgeless_api::function_instance::NodeId, crate::controller::server::PeerCluster>>,
    >,
}

impl DefaultPlacement {
    pub fn new(
        orchestration_logic: std::sync::Arc<tokio::sync::Mutex<crate::orchestration_logic::OrchestrationLogic>>,
        nodes: std::sync::Arc<
            tokio::sync::Mutex<std::collections::HashMap<edgeless_api::function_instance::NodeId, crate::controller::server::WorkerNode>>,
        >,
        peer_clusters: std::sync::Arc<
            tokio::sync::Mutex<std::collections::HashMap<edgeless_api::function_instance::NodeId, crate::controller::server::PeerCluster>>,
        >,
    ) -> Self {
        Self {
            orchestration_logic,
            nodes,
            peer_clusters,
        }
    }
}

impl super::Transformation for DefaultPlacement {
    fn apply(&mut self, slf: &mut workflow::ActiveWorkflow) {
        for (f_id, function) in &mut slf.functions {
            let mut function = function.borrow_mut();
            if function.instances.is_empty() {
                let dst = self
                    .orchestration_logic
                    .blocking_lock()
                    .next(&self.nodes.blocking_lock(), &function.image.format, &function.annotations);

                if let Some(dst) = dst {
                    function.instances.push(std::cell::RefCell::new(actor::PhysicalActor {
                        id: edgeless_api::function_instance::InstanceId::new(dst),
                        desired_mapping: PhysicalPorts::default(),
                        image: None,
                        materialized: None,
                    }))
                } else {
                    log::info!("Found no viable node for {} in {}", &f_id, slf.id.workflow_id);
                }
            }
        }

        for (_, resource) in &mut slf.resources {
            let mut resource = resource.borrow_mut();
            if resource.instances.is_empty() {
                let dst = select_node_for_resource(&resource, &self.nodes.blocking_lock());
                if let Some(dst) = dst {
                    resource.instances.push(std::cell::RefCell::new(resource::PhysicalResource {
                        id: edgeless_api::function_instance::InstanceId::new(dst),
                        desired_mapping: PhysicalPorts::default(),
                        materialized: None,
                    }));
                }
            }
        }

        for (_, subflow) in &mut slf.subflows {
            let mut subflow = subflow.borrow_mut();
            if subflow.instances.is_empty() {
                if subflow.instances.is_empty() {
                    let dst = select_cluster_for_subflow(&subflow, &self.peer_clusters.blocking_lock());
                    if let Some(dst) = dst {
                        subflow.instances.push(std::cell::RefCell::new(subflow::PhysicalSubFlow {
                            id: edgeless_api::function_instance::InstanceId::new(dst),
                            desired_mapping: PhysicalPorts::default(),
                            materialized: None,
                        }))
                    }
                }
            }
        }

        {
            let mut proxy = slf.proxy.borrow_mut();
            if !proxy.logical_ports.logical_input_mapping.is_empty() || !proxy.logical_ports.logical_output_mapping.is_empty() {
                if proxy.instances.is_empty() {
                    let dst = select_node_for_proxy(&proxy, &self.nodes.blocking_lock());
                    if let Some(dst) = dst {
                        proxy.instances.push(std::cell::RefCell::new(proxy::PhyiscalProxy {
                            id: edgeless_api::function_instance::InstanceId::new(dst),
                            desired_mapping: PhysicalPorts::default(),
                            materialized: None,
                        }));
                    }
                }
            }
        }
    }
}

fn select_node_for_resource(
    resource: &resource::LogicalResource,
    nodes: &std::collections::HashMap<edgeless_api::function_instance::NodeId, crate::controller::server::WorkerNode>,
) -> Option<edgeless_api::function_instance::NodeId> {
    if let Some((id, _)) = nodes
        .iter()
        .find(|(_, n)| n.resource_providers.iter().find(|(_, r)| r.class_type == resource.class).is_some())
    {
        Some(id.clone())
    } else {
        None
    }
}

fn select_node_for_proxy(
    _proxy: &proxy::LogicalProxy,
    nodes: &std::collections::HashMap<edgeless_api::function_instance::NodeId, crate::controller::server::WorkerNode>,
) -> Option<edgeless_api::function_instance::NodeId> {
    for (node_id, node) in nodes {
        if node.is_proxy {
            return Some(node_id.clone());
        }
    }
    return None;
}

fn select_cluster_for_subflow(
    subflow: &subflow::LogicalSubFlow,
    clusters: &std::collections::HashMap<edgeless_api::function_instance::NodeId, crate::controller::server::PeerCluster>,
) -> Option<edgeless_api::function_instance::NodeId> {
    for (cluster_id, cluster) in clusters {
        // TODO Proper Selection
        return Some(cluster_id.clone());
    }
    return None;
}
