// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

use crate::ir::interaction::dialect::{
    AsConcreteDestinationPort, AsConcreteDialectConstraint, AsConcreteInteraction, AsConcreteSourcePort, DialectConstraint,
};

pub static ID: super::DialectId = super::DialectId("IP_MULTICAST");

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum IpMulticastConstraint {
    Cluster(uuid::Uuid),
}

impl super::DialectConstraint for IpMulticastConstraint {
    fn as_container(self) -> super::DialectConstraintContainer {
        super::DialectConstraintContainer {
            dialect: ID.clone(),
            constraint: Box::new(self),
        }
    }
}

pub struct IpMulticastDialect {
    state: std::sync::Arc<tokio::sync::Mutex<DialectState>>,
}

struct DialectState {
    pool_free: Vec<std::net::Ipv4Addr>,
    active: std::collections::HashMap<edgeless_api::link::LinkInstanceId, IpMulticastInteraction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpMulticastSourcePort {
    pub link_id: edgeless_api::link::LinkInstanceId,
    multicast_ip: std::net::Ipv4Addr,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpMulticastDestinationPort {
    pub link_id: edgeless_api::link::LinkInstanceId,
    multicast_ip: std::net::Ipv4Addr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpMulticastInteraction {
    pub link_id: edgeless_api::link::LinkInstanceId,
    pub multicast_ip: std::net::Ipv4Addr,
    pub publishers: Vec<crate::ir::interaction::PhysicalPortId>,
    pub subscribers: Vec<crate::ir::interaction::PhysicalPortId>,
}

impl super::DestinationPort for IpMulticastDestinationPort {}
impl super::SourcePort for IpMulticastSourcePort {}
impl super::Interaction for IpMulticastInteraction {
    fn as_physical(&self) -> Option<&dyn super::PhysicalInteraction> {
        return Some(self);
    }
}

impl super::PhysicalInteraction for IpMulticastInteraction {
    fn relevant_nodes(&self) -> Vec<edgeless_api::function_instance::NodeId> {
        let mut nodes = std::collections::BTreeSet::new();
        for p in &self.publishers {
            nodes.insert(p.instance.node_id);
        }

        for s in &self.subscribers {
            nodes.insert(s.instance.node_id);
        }

        nodes.into_iter().collect()
    }
}

impl super::InteractionPortUtils<crate::ir::interaction::PhysicalPortId> for IpMulticastDialect {
    fn ports_to_interaction(
        &self,
        srcs: Vec<(crate::ir::interaction::PhysicalPortId, super::super::SourcePortMapping)>,
        dests: Vec<(crate::ir::interaction::PhysicalPortId, super::super::DestiantionPortMapping)>,
    ) -> Result<Vec<super::super::InteractionMapping>, crate::ir::interaction::InteractionError> {
        let mut buffer = std::collections::HashMap::<edgeless_api::link::LinkInstanceId, IpMulticastInteraction>::new();

        for (port_id, src) in srcs {
            let multicast_mapping = IpMulticastSourcePort::as_concrete(src.mapping.as_ref())?;
            buffer
                .entry(multicast_mapping.link_id.clone())
                .or_insert(IpMulticastInteraction {
                    link_id: multicast_mapping.link_id.clone(),
                    multicast_ip: multicast_mapping.multicast_ip,
                    publishers: Vec::new(),
                    subscribers: Vec::new(),
                })
                .publishers
                .push(port_id);
        }

        for (port_id, dst) in dests {
            let multicast_mapping = IpMulticastDestinationPort::as_concrete(dst.mapping.as_ref())?;
            buffer
                .entry(multicast_mapping.link_id.clone())
                .or_insert(IpMulticastInteraction {
                    link_id: multicast_mapping.link_id.clone(),
                    multicast_ip: multicast_mapping.multicast_ip,
                    publishers: Vec::new(),
                    subscribers: Vec::new(),
                })
                .subscribers
                .push(port_id);
        }

        Ok(buffer
            .into_values()
            .map(|i| super::super::InteractionMapping {
                mapping: Box::new(i),
                dialect_type: super::DialectDescriptor {
                    base_type: ID,
                    constraints: std::collections::BTreeSet::new(),
                },
            })
            .collect())
    }

    fn interaction_to_ports(
        &self,
        interaction: super::super::InteractionMapping,
    ) -> Result<
        (
            Vec<(crate::ir::interaction::PhysicalPortId, super::super::SourcePortMapping)>,
            Vec<(crate::ir::interaction::PhysicalPortId, super::super::DestiantionPortMapping)>,
        ),
        crate::ir::interaction::InteractionError,
    > {
        let multicast_mapping = IpMulticastInteraction::as_concrete(interaction.mapping.as_ref())?;

        let mut subscribers = Vec::new();
        let mut publishers = Vec::new();

        for publisher in &multicast_mapping.publishers {
            publishers.push((
                publisher.clone(),
                super::super::SourcePortMapping {
                    mapping: Box::new(IpMulticastSourcePort {
                        link_id: multicast_mapping.link_id.clone(),
                        multicast_ip: multicast_mapping.multicast_ip.clone(),
                    }),
                    dialect_type: super::DialectDescriptor {
                        base_type: ID,
                        constraints: std::collections::BTreeSet::new(),
                    },
                },
            ));
        }

        for subscriber in &multicast_mapping.subscribers {
            subscribers.push((
                subscriber.clone(),
                super::super::DestiantionPortMapping {
                    mapping: Box::new(IpMulticastDestinationPort {
                        link_id: multicast_mapping.link_id.clone(),
                        multicast_ip: multicast_mapping.multicast_ip.clone(),
                    }),
                    dialect_type: super::DialectDescriptor {
                        base_type: ID,
                        constraints: std::collections::BTreeSet::new(),
                    },
                },
            ));
        }

        Ok((publishers, subscribers))
    }
}

impl super::InteractionDialect for IpMulticastDialect {
    fn id(&self) -> super::DialectId {
        ID
    }

    fn provides_transformations_to(&self) -> &'static [super::DialectId] {
        &[]
    }

    fn provides_transformations_from(&self) -> &'static [super::DialectId] {
        static TRANSFORMATIONS: [super::DialectId; 1] = [super::physical_overlay::ID];
        &TRANSFORMATIONS
    }

    fn plan_translation_to(
        &self,
        src: &super::super::InteractionMapping,
        dst: &super::DialectDescriptor,
    ) -> Result<(super::DialectDescriptor, u64), super::super::InteractionError> {
        if src.dialect_type.base_type == super::physical_overlay::ID && dst.base_type == ID {
            let constraints = src
                .dialect_type
                .constraints
                .iter()
                .filter_map(|c| {
                    if let Ok(physical_overlay_constraint) = super::physical_overlay::PhysicalOverlayConstraint::from_container(&c) {
                        match physical_overlay_constraint {
                            super::physical_overlay::PhysicalOverlayConstraint::Cluster(uuid) => {
                                Some(IpMulticastConstraint::Cluster(uuid.clone()).as_container())
                            }
                        }
                    } else {
                        None
                    }
                })
                .collect();

            return Ok((
                super::DialectDescriptor {
                    base_type: ID,
                    constraints: constraints,
                },
                2,
            ));
        }

        Err(super::super::InteractionError::UnsupportedTranslation(
            src.dialect_type.clone(),
            dst.clone(),
        ))
    }

    fn execute_transformation_to(
        &self,
        src: &crate::ir::interaction::InteractionMapping,
        dest: &super::DialectDescriptor,
    ) -> Result<Vec<crate::ir::interaction::InteractionMapping>, crate::ir::interaction::InteractionError> {
        let mut state = self.state.blocking_lock();

        if src.dialect_type.base_type != super::physical_overlay::ID || dest.base_type != ID {
            return Err(crate::ir::interaction::InteractionError::UnsupportedTranslation(
                src.dialect_type.clone(),
                dest.clone(),
            ));
        }

        let overlay_src = super::physical_overlay::PhyscialOverlayInteraction::as_concrete(src.mapping.as_ref())?;

        if let super::physical_overlay::DestinationMapping::Multicast(destinations) = &overlay_src.destination {
            // Keep behavior consistent for now
            if destinations.len() < 2 {
                return Err(crate::ir::interaction::InteractionError::Inefficient);
            }

            for (_, active_multicast_link) in &mut state.active {
                let existing_publishers = std::collections::BTreeSet::from_iter(active_multicast_link.publishers.iter());
                let existing_subscribers = std::collections::BTreeSet::from_iter(active_multicast_link.subscribers.iter());

                let new_publishers = std::collections::BTreeSet::from_iter(overlay_src.sources.iter());
                let new_subscribers = std::collections::BTreeSet::from_iter(destinations.iter());

                // Keep behavior nearly consistent with old version
                if existing_publishers == new_publishers && existing_subscribers.is_subset(&new_subscribers) {
                    active_multicast_link.publishers = destinations.clone();
                    return Ok(vec![super::super::InteractionMapping {
                        dialect_type: dest.clone(),
                        mapping: Box::new(active_multicast_link.clone()),
                    }]);
                }
            }

            let id = edgeless_api::link::LinkInstanceId(uuid::Uuid::new_v4());
            let ip = state.pool_free.pop();

            if let Some(ip) = ip {
                let interaction = IpMulticastInteraction {
                    link_id: id.clone(),
                    multicast_ip: ip,
                    publishers: overlay_src.sources.clone(),
                    subscribers: destinations.clone(),
                };

                state.active.insert(id.clone(), interaction.clone());
                return Ok(vec![super::super::InteractionMapping {
                    dialect_type: dest.clone(),
                    mapping: Box::new(interaction),
                }]);
            }
        }
        Err(crate::ir::interaction::InteractionError::LinkCapacity)
    }

    fn logical_utils(&self) -> Option<&dyn super::InteractionPortUtils<crate::ir::interaction::LogicalPortId>> {
        return None;
    }

    fn physical_utils(&self) -> Option<&dyn super::InteractionPortUtils<crate::ir::interaction::PhysicalPortId>> {
        return Some(self);
    }

    fn link_config(
        &self,
        mapping: &crate::ir::interaction::InteractionMapping,
        nodes: &std::collections::HashMap<uuid::Uuid, &dyn crate::ir::Node>,
    ) -> super::super::LinkConfigurationResult {
        let mcast_link_id = edgeless_api::link::LinkType("MULTICAST".to_string());

        let mcast_mapping = match IpMulticastInteraction::as_concrete(mapping.mapping.as_ref()) {
            Ok(mcast_mapping) => mcast_mapping,
            Err(e) => {
                return super::super::LinkConfigurationResult::Err(e);
            }
        };

        let mut relevant_nodes = std::collections::BTreeSet::new();

        for sub in &mcast_mapping.subscribers {
            relevant_nodes.insert(sub.instance.node_id.clone());
        }

        for publisher in &mcast_mapping.publishers {
            relevant_nodes.insert(publisher.instance.node_id.clone());
        }

        let relevant_nodes: Result<Vec<_>, crate::ir::interaction::InteractionError> = relevant_nodes
            .into_iter()
            .map(|n| {
                Ok((
                    n.clone(),
                    nodes
                        .get(&n)
                        .ok_or(crate::ir::interaction::InteractionError::LinkConfiguration(anyhow::anyhow!(
                            "Corresponding node not found."
                        )))?
                        .available_link_types()
                        .get(&mcast_link_id)
                        .ok_or(crate::ir::interaction::InteractionError::LinkConfiguration(anyhow::anyhow!(
                            "Corresponding link provider not found."
                        )))?
                        .clone(),
                    self.config_for(mcast_mapping.link_id.clone(), n)?,
                    false,
                ))
            })
            .collect();

        let relevant_nodes = match relevant_nodes {
            Ok(relevant_nodes) => relevant_nodes,
            Err(e) => return crate::ir::interaction::LinkConfigurationResult::Err(e),
        };

        crate::ir::interaction::LinkConfigurationResult::Ok(crate::ir::link::WorkflowLink {
            id: mcast_mapping.link_id.clone(),
            class: mcast_link_id.clone(),
            materialized: false,
            nodes: relevant_nodes,
        })
    }
}

impl IpMulticastDialect {
    pub fn new() -> Self {
        let pool_free: Vec<_> = std::ops::Range { start: 153, end: 253 }
            .map(|i| std::net::Ipv4Addr::new(224, 0, 0, i))
            .collect();

        Self {
            state: std::sync::Arc::new(tokio::sync::Mutex::new(DialectState {
                pool_free,
                active: std::collections::HashMap::new(),
            })),
        }
    }

    pub fn config_for(
        &self,
        link: edgeless_api::link::LinkInstanceId,
        _node: edgeless_api::function_instance::NodeId,
    ) -> Result<Vec<u8>, crate::ir::interaction::InteractionError> {
        let state = self.state.blocking_lock();

        if let Some(active_link) = state.active.get(&link) {
            let cfg = edgeless_link_multicast::common::MulticastConfig {
                ip: active_link.multicast_ip,
                port: 9999,
            };
            Ok(serde_json::to_string(&cfg)
                .map_err(|e| crate::ir::interaction::InteractionError::LinkConfiguration(e.into()))?
                .into_bytes())
        } else {
            Err(crate::ir::interaction::InteractionError::LinkConfiguration(anyhow::anyhow!(
                "Link not found."
            )))
        }
    }
}
