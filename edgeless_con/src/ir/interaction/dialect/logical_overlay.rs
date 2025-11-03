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

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum LogicalOverlayConstraint {}

pub struct LogicalOverlayDialect {}

impl super::DestinationPort for LogicalOverlayDestinationPort {}
impl super::SourcePort for LogicalOverlaySourcePort {}
impl super::Interaction for LogicalOverlayInteraction {
    fn as_physical(&self) -> Option<&dyn super::PhysicalInteraction> {
        None
    }
}

impl super::InteractionPortUtils<crate::ir::interaction::LogicalPortId> for LogicalOverlayDialect {
    fn ports_to_interaction(
        &self,
        srcs: Vec<(crate::ir::interaction::LogicalPortId, super::super::SourcePortMapping)>,
        _dests: Vec<(crate::ir::interaction::LogicalPortId, super::super::DestiantionPortMapping)>,
    ) -> Vec<super::super::InteractionMapping> {
        let mut collector = std::collections::BTreeMap::<DestinationMapping, Vec<crate::ir::interaction::LogicalPortId>>::new();

        for (src_port_id, src_spec) in srcs {
            let any_mapping = src_spec.mapping.as_ref() as &dyn std::any::Any;
            let maybe_overlay_mapping = any_mapping.downcast_ref::<LogicalOverlaySourcePort>();
            let overlay_mapping = maybe_overlay_mapping.unwrap();
            collector.entry(overlay_mapping.destination.clone()).or_default().push(src_port_id)
        }

        collector
            .into_iter()
            .map(|(destination, sources)| super::super::InteractionMapping {
                mapping: Box::new(LogicalOverlayInteraction { sources, destination }),
                dialect_type: super::DialectDescriptor {
                    base_type: ID,
                    constraints: std::collections::BTreeSet::new(),
                },
            })
            .collect()
    }

    fn interaction_to_ports(
        &self,
        interaction: super::super::InteractionMapping,
    ) -> (
        Vec<(crate::ir::interaction::LogicalPortId, super::super::SourcePortMapping)>,
        Vec<(crate::ir::interaction::LogicalPortId, super::super::DestiantionPortMapping)>,
    ) {
        let any_mapping = interaction.mapping.as_ref() as &dyn std::any::Any;
        let maybe_overlay_mapping = any_mapping.downcast_ref::<LogicalOverlayInteraction>();
        let overlay_mapping = maybe_overlay_mapping.unwrap();

        let mut srcs = Vec::new();
        let mut dests = Vec::new();

        for src_port_id in overlay_mapping.sources.clone() {
            srcs.push((
                src_port_id,
                super::super::SourcePortMapping {
                    mapping: Box::new(LogicalOverlaySourcePort {
                        destination: overlay_mapping.destination.clone(),
                    }),
                    dialect_type: super::DialectDescriptor {
                        base_type: ID,
                        constraints: std::collections::BTreeSet::new(),
                    },
                },
            ));
        }

        match &overlay_mapping.destination {
            DestinationMapping::Unicast(dest_port_id) => {
                dests.push((
                    dest_port_id.clone(),
                    LogicalOverlayDestinationPort {
                        sources: overlay_mapping.sources.clone(),
                    },
                ));
            }
            DestinationMapping::Anycast(dest_port_ids) => {
                for dest_port_id in dest_port_ids {
                    dests.push((
                        dest_port_id.clone(),
                        LogicalOverlayDestinationPort {
                            sources: overlay_mapping.sources.clone(),
                        },
                    ));
                }
            }
            DestinationMapping::Multicast(dest_port_ids) => {
                for dest_port_id in dest_port_ids {
                    dests.push((
                        dest_port_id.clone(),
                        LogicalOverlayDestinationPort {
                            sources: overlay_mapping.sources.clone(),
                        },
                    ));
                }
            }
        }

        let dests = dests
            .into_iter()
            .map(|(id, mapping)| {
                (
                    id,
                    super::super::DestiantionPortMapping {
                        dialect_type: super::DialectDescriptor {
                            base_type: ID,
                            constraints: std::collections::BTreeSet::new(),
                        },
                        mapping: Box::new(mapping),
                    },
                )
            })
            .collect();

        (srcs, dests)
    }
}

impl super::InteractionDialect for LogicalOverlayDialect {
    fn id(&self) -> super::DialectId {
        return ID;
    }

    fn provides_transformations_to(&self) -> &'static [super::DialectId] {
        // Logical->Physical is handled by a special transformation
        &[]
    }

    fn provides_transformations_from(&self) -> &'static [super::DialectId] {
        &[]
    }

    fn plan_translation_to(&self, _src: &super::DialectDescriptor, _dst: &super::DialectDescriptor) -> Result<super::DialectDescriptor, ()> {
        Err(())
    }

    fn execute_transformation_to(
        &self,
        _src: &crate::ir::interaction::InteractionMapping,
        _dest: &super::DialectDescriptor,
    ) -> Result<Vec<crate::ir::interaction::InteractionMapping>, ()> {
        Err(())
    }

    fn logical_utils(&self) -> Option<&dyn super::InteractionPortUtils<crate::ir::interaction::LogicalPortId>> {
        return Some(self);
    }

    fn physical_utils(&self) -> Option<&dyn super::InteractionPortUtils<crate::ir::interaction::PhysicalPortId>> {
        return None;
    }

    fn link_config(
        &self,
        _mapping: &crate::ir::interaction::InteractionMapping,
        _nodes: &std::collections::HashMap<uuid::Uuid, &dyn crate::ir::Node>,
    ) -> crate::ir::link::WorkflowLink {
        todo!()
    }
}
