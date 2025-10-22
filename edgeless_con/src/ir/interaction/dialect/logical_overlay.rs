// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

pub static ID: super::DialectId = super::DialectId("LOGICAL_OVERLAY");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicalOverlaySourcePort {
    pub destination: DestinationMapping,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicalOverlayDestinationPort {
    pub sources: Vec<crate::ir::interaction::LogicalPortId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicalOverlayInteraction {
    pub sources: Vec<crate::ir::interaction::LogicalPortId>,
    pub destination: DestinationMapping,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum DestinationMapping {
    Unicast(crate::ir::interaction::LogicalPortId),
    Anycast(Vec<crate::ir::interaction::LogicalPortId>),
    Multicast(Vec<crate::ir::interaction::LogicalPortId>),
}

#[derive(Debug)]
pub enum LogicalOverlayConstraint {
    None,
}

pub struct LogicalOverlayDialect {}

impl super::DestinationPort for LogicalOverlayDestinationPort {}
impl super::SourcePort for LogicalOverlaySourcePort {}
impl super::Interaction for LogicalOverlayInteraction {}

impl
    super::InteractionDialect<
        crate::ir::interaction::LogicalPortId,
        LogicalOverlaySourcePort,
        LogicalOverlayDestinationPort,
        LogicalOverlayInteraction,
    > for LogicalOverlayDialect
{
    fn id(&self) -> super::DialectId {
        return ID;
    }

    fn provides_transformations_to(&self) -> &'static [super::DialectId] {
        static TRANSFORMATIONS: [super::DialectId; 2] = [super::ip_multicast::ID, super::physical_overlay::ID];
        &TRANSFORMATIONS
    }

    fn plan_translation_to(&self, src: super::DialectDescriptor, dst: super::DialectDescriptor) -> Result<super::DialectDescriptor, ()> {
        todo!()
    }

    fn ports_to_interaction(
        &self,
        srcs: Vec<(crate::ir::interaction::LogicalPortId, LogicalOverlaySourcePort)>,
        _dests: Vec<(crate::ir::interaction::LogicalPortId, LogicalOverlayDestinationPort)>,
    ) -> Vec<LogicalOverlayInteraction> {
        let mut collector = std::collections::BTreeMap::<DestinationMapping, Vec<crate::ir::interaction::LogicalPortId>>::new();

        for (src_port_id, src_spec) in srcs {
            collector.entry(src_spec.destination).or_default().push(src_port_id)
        }

        collector
            .into_iter()
            .map(|(destination, sources)| LogicalOverlayInteraction { sources, destination })
            .collect()
    }

    fn interaction_to_ports(
        &self,
        interaction: LogicalOverlayInteraction,
    ) -> (
        Vec<(crate::ir::interaction::LogicalPortId, LogicalOverlaySourcePort)>,
        Vec<(crate::ir::interaction::LogicalPortId, LogicalOverlayDestinationPort)>,
    ) {
        let mut srcs = Vec::new();
        let mut dests = Vec::new();

        for src_port_id in interaction.sources.clone() {
            srcs.push((
                src_port_id,
                LogicalOverlaySourcePort {
                    destination: interaction.destination.clone(),
                },
            ));
        }

        match interaction.destination {
            DestinationMapping::Unicast(dest_port_id) => {
                dests.push((
                    dest_port_id,
                    LogicalOverlayDestinationPort {
                        sources: interaction.sources.clone(),
                    },
                ));
            }
            DestinationMapping::Anycast(dest_port_ids) => {
                for dest_port_id in dest_port_ids {
                    dests.push((
                        dest_port_id,
                        LogicalOverlayDestinationPort {
                            sources: interaction.sources.clone(),
                        },
                    ));
                }
            }
            DestinationMapping::Multicast(dest_port_ids) => {
                for dest_port_id in dest_port_ids {
                    dests.push((
                        dest_port_id,
                        LogicalOverlayDestinationPort {
                            sources: interaction.sources.clone(),
                        },
                    ));
                }
            }
        }

        (srcs, dests)
    }
}
