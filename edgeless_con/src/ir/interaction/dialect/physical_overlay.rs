// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

pub static ID: super::DialectId = super::DialectId("PHYSICAL_OVERLAY");

pub enum PhysicalOverlayConstraint {
    Cluster(uuid::Uuid),
}

pub struct PhysicalOverlayDialect {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhysicalOverlaySourcePort {
    pub destination: DestinationMapping,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhysicalOverlayDestinationPort {
    pub sources: Vec<crate::ir::interaction::PhysicalPortId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhyscialOverlayInteraction {
    pub destination: DestinationMapping,
    pub sources: Vec<crate::ir::interaction::PhysicalPortId>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum DestinationMapping {
    Unicast(crate::ir::interaction::PhysicalPortId),
    Anycast(Vec<crate::ir::interaction::PhysicalPortId>),
    Multicast(Vec<crate::ir::interaction::PhysicalPortId>),
}

impl super::DestinationPort for PhysicalOverlayDestinationPort {}
impl super::SourcePort for PhysicalOverlaySourcePort {}
impl super::Interaction for PhyscialOverlayInteraction {}

impl
    super::InteractionDialect<
        crate::ir::interaction::PhysicalPortId,
        PhysicalOverlaySourcePort,
        PhysicalOverlayDestinationPort,
        PhyscialOverlayInteraction,
    > for PhysicalOverlayDialect
{
    fn id(&self) -> super::DialectId {
        ID
    }

    fn provides_transformations_to(&self) -> &'static [super::DialectId] {
        static TRANSFORMATIONS: [super::DialectId; 1] = [super::ip_multicast::ID];
        &TRANSFORMATIONS
    }

    fn plan_translation_to(&self, src: super::DialectDescriptor, dst: super::DialectDescriptor) -> Result<super::DialectDescriptor, ()> {
        todo!()
    }

    fn ports_to_interaction(
        &self,
        srcs: Vec<(crate::ir::interaction::PhysicalPortId, PhysicalOverlaySourcePort)>,
        _dests: Vec<(crate::ir::interaction::PhysicalPortId, PhysicalOverlayDestinationPort)>,
    ) -> Vec<PhyscialOverlayInteraction> {
        let mut collector = std::collections::BTreeMap::<DestinationMapping, Vec<crate::ir::interaction::PhysicalPortId>>::new();

        for (src_port_id, src_spec) in srcs {
            collector.entry(src_spec.destination).or_default().push(src_port_id)
        }

        collector
            .into_iter()
            .map(|(destination, sources)| PhyscialOverlayInteraction { sources, destination })
            .collect()
    }

    fn interaction_to_ports(
        &self,
        interaction: PhyscialOverlayInteraction,
    ) -> (
        Vec<(crate::ir::interaction::PhysicalPortId, PhysicalOverlaySourcePort)>,
        Vec<(crate::ir::interaction::PhysicalPortId, PhysicalOverlayDestinationPort)>,
    ) {
        let mut srcs = Vec::new();
        let mut dests = Vec::new();

        for src_port_id in interaction.sources.clone() {
            srcs.push((
                src_port_id,
                PhysicalOverlaySourcePort {
                    destination: interaction.destination.clone(),
                },
            ));
        }

        match interaction.destination {
            DestinationMapping::Unicast(dest_port_id) => {
                dests.push((
                    dest_port_id,
                    PhysicalOverlayDestinationPort {
                        sources: interaction.sources.clone(),
                    },
                ));
            }
            DestinationMapping::Anycast(dest_port_ids) => {
                for dest_port_id in dest_port_ids {
                    dests.push((
                        dest_port_id,
                        PhysicalOverlayDestinationPort {
                            sources: interaction.sources.clone(),
                        },
                    ));
                }
            }
            DestinationMapping::Multicast(dest_port_ids) => {
                for dest_port_id in dest_port_ids {
                    dests.push((
                        dest_port_id,
                        PhysicalOverlayDestinationPort {
                            sources: interaction.sources.clone(),
                        },
                    ));
                }
            }
        }

        (srcs, dests)
    }
}
