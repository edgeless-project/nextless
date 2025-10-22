// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

pub static ID: super::DialectId = super::DialectId("IP_MULTICAST");

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IpMulticastConstraint {
    Cluster(uuid::Uuid),
}

pub struct IpMulticastDialect {
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
impl super::Interaction for IpMulticastInteraction {}

impl super::InteractionDialect<crate::ir::interaction::PhysicalPortId, IpMulticastSourcePort, IpMulticastDestinationPort, IpMulticastInteraction>
    for IpMulticastDialect
{
    fn id(&self) -> super::DialectId {
        ID
    }

    fn provides_transformations_to(&self) -> &'static [super::DialectId] {
        &[]
    }

    fn plan_translation_to(&self, src: super::DialectDescriptor, dst: super::DialectDescriptor) -> Result<super::DialectDescriptor, ()> {
        todo!()
    }

    fn ports_to_interaction(
        &self,
        srcs: Vec<(crate::ir::interaction::PhysicalPortId, IpMulticastSourcePort)>,
        dests: Vec<(crate::ir::interaction::PhysicalPortId, IpMulticastDestinationPort)>,
    ) -> Vec<IpMulticastInteraction> {
        let mut buffer = std::collections::HashMap::<edgeless_api::link::LinkInstanceId, IpMulticastInteraction>::new();

        for (port_id, src) in srcs {
            buffer
                .entry(src.link_id.clone())
                .or_insert(IpMulticastInteraction {
                    link_id: src.link_id,
                    multicast_ip: src.multicast_ip,
                    publishers: Vec::new(),
                    subscribers: Vec::new(),
                })
                .publishers
                .push(port_id);
        }

        for (port_id, dst) in dests {
            buffer
                .entry(dst.link_id.clone())
                .or_insert(IpMulticastInteraction {
                    link_id: dst.link_id,
                    multicast_ip: dst.multicast_ip,
                    publishers: Vec::new(),
                    subscribers: Vec::new(),
                })
                .subscribers
                .push(port_id);
        }

        buffer.into_values().collect()
    }

    fn interaction_to_ports(
        &self,
        interaction: IpMulticastInteraction,
    ) -> (
        Vec<(crate::ir::interaction::PhysicalPortId, IpMulticastSourcePort)>,
        Vec<(crate::ir::interaction::PhysicalPortId, IpMulticastDestinationPort)>,
    ) {
        let mut subscribers = Vec::new();
        let mut publishers = Vec::new();

        for publisher in &interaction.publishers {
            publishers.push((
                publisher.clone(),
                IpMulticastSourcePort {
                    link_id: interaction.link_id.clone(),
                    multicast_ip: interaction.multicast_ip.clone(),
                },
            ));
        }

        for subscriber in &interaction.subscribers {
            subscribers.push((
                subscriber.clone(),
                IpMulticastDestinationPort {
                    link_id: interaction.link_id.clone(),
                    multicast_ip: interaction.multicast_ip.clone(),
                },
            ));
        }

        (publishers, subscribers)
    }
}

impl IpMulticastDialect {
    pub fn new() -> Self {
        let pool_free: Vec<_> = std::ops::Range { start: 153, end: 253 }
            .map(|i| std::net::Ipv4Addr::new(224, 0, 0, i))
            .collect();

        Self {
            pool_free,
            active: std::collections::HashMap::new(),
        }
    }

    pub fn translate_from_physial_overlay(&mut self, src: super::physical_overlay::PhyscialOverlayInteraction) -> Option<IpMulticastInteraction> {
        if let super::physical_overlay::DestinationMapping::Multicast(destinations) = src.destination {
            // Keep behavior consistent for now
            if destinations.len() < 2 {
                return None;
            }

            for (_, active_multicast_link) in &mut self.active {
                let existing_publishers = std::collections::BTreeSet::from_iter(active_multicast_link.publishers.iter());
                let existing_subscribers = std::collections::BTreeSet::from_iter(active_multicast_link.subscribers.iter());

                let new_publishers = std::collections::BTreeSet::from_iter(src.sources.iter());
                let new_subscribers = std::collections::BTreeSet::from_iter(destinations.iter());

                // Keep behavior nearly consistent with old version
                if existing_publishers == new_publishers && existing_subscribers.is_subset(&new_subscribers) {
                    active_multicast_link.publishers = destinations.clone();
                    return Some(active_multicast_link.clone());
                }
            }

            let id = edgeless_api::link::LinkInstanceId(uuid::Uuid::new_v4());
            let ip = self.pool_free.pop();

            if let Some(ip) = ip {
                let interaction = IpMulticastInteraction {
                    link_id: id.clone(),
                    multicast_ip: ip,
                    publishers: src.sources,
                    subscribers: destinations,
                };

                self.active.insert(id.clone(), interaction.clone());
                return Some(interaction);
            }
        }
        None
    }

    pub fn config_for(&self, link: edgeless_api::link::LinkInstanceId, _node: edgeless_api::function_instance::NodeId) -> Option<Vec<u8>> {
        if let Some(active_link) = self.active.get(&link) {
            let cfg = edgeless_link_multicast::common::MulticastConfig {
                ip: active_link.multicast_ip,
                port: 9999,
            };
            Some(serde_json::to_string(&cfg).unwrap().into_bytes())
        } else {
            None
        }
    }
}
