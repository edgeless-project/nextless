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
            } else {
                // This is a test that shows the use of the Materialized Representation
                for i in &function.instances() {
                    let instance = i.borrow_mut();
                    if let Some(materialized) = &instance.materialized_state() {
                        let mut materialized = materialized.borrow_mut();
                        if let Some(v) = materialized
                            .runtime_statistics()
                            .and_then(|s| s.invocation_rate_abs(std::time::Duration::from_secs(120)))
                        {
                            log::debug!("Invocation Rate: {}", v)
                        }
                        if let Some(v) = materialized
                            .runtime_statistics()
                            .and_then(|s| Some(s.invocations_rate_abs_by_port(std::time::Duration::from_secs(120))))
                        {
                            log::debug!("Invocation Rate By Port: {:?}", v);
                        }

                        if let Some(v) = materialized
                            .runtime_statistics()
                            .and_then(|s| s.duration_mean_secs(std::time::Duration::from_secs(120)))
                        {
                            log::debug!("Mean Duration MS: {}", v * 1000.0);
                        }
                        if let Some(v) = materialized
                            .runtime_statistics()
                            .and_then(|s| s.duration_soft_limit_rate_rel(std::time::Duration::from_secs(120)))
                        {
                            log::debug!("Duration Soft Limit Score: {}", v);
                        }

                        if let Some(v) = materialized
                            .runtime_statistics()
                            .and_then(|s| s.error_rate_rel(std::time::Duration::from_secs(120)))
                        {
                            log::debug!("Error Rate {}", v);
                        }

                        for (p_id, p) in &materialized.materialized_ports().materialized_inputs {
                            if let Some(v) = p
                                .runtime_statistics()
                                .and_then(|s| s.message_rate_abs(std::time::Duration::from_secs(120)))
                            {
                                log::debug!("Port Rate {}: {}", p_id.0, v);
                            }
                            if let Some(v) = p
                                .runtime_statistics()
                                .and_then(|s| Some(s.message_rate_abs_by_peer(std::time::Duration::from_secs(120))))
                            {
                                log::debug!("Port By Peer Rate {}: {:?}", p_id.0, v);
                            }
                            if let Some(v) = p
                                .runtime_statistics()
                                .and_then(|s| s.message_size_mean_bytes(std::time::Duration::from_secs(120)))
                            {
                                log::debug!("Port Size {}: {}", p_id.0, v);
                            }
                            if let Some(v) = p
                                .runtime_statistics()
                                .and_then(|s| Some(s.message_size_mean_byte_by_peer(std::time::Duration::from_secs(120))))
                            {
                                log::debug!("Port By Peer Size {}: {:?}", p_id.0, v);
                            }
                        }
                    }
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
            if subflow.instances.is_empty() && subflow.instances.is_empty() {
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

        {
            let mut proxy = slf.proxy.borrow_mut();
            if (!proxy.logical_ports.logical_input_mapping.is_empty() || !proxy.logical_ports.logical_output_mapping.is_empty())
                && proxy.instances.is_empty()
            {
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

fn select_node_for_resource(
    resource: &resource::LogicalResource,
    nodes: &std::collections::HashMap<edgeless_api::function_instance::NodeId, crate::controller::server::WorkerNode>,
) -> Option<edgeless_api::function_instance::NodeId> {
    if let Some((id, _)) = nodes
        .iter()
        .find(|(_, n)| n.resource_providers.iter().any(|(_, r)| r.class_type == resource.class))
    {
        Some(*id)
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
            return Some(*node_id);
        }
    }
    None
}

fn select_cluster_for_subflow(
    subflow: &subflow::LogicalSubFlow,
    clusters: &std::collections::HashMap<edgeless_api::function_instance::NodeId, crate::controller::server::PeerCluster>,
) -> Option<edgeless_api::function_instance::NodeId> {
    // for (cluster_id, cluster) in clusters {
    //     // TODO Proper Selection
    //     return Some(*cluster_id);
    // }
    None
}
