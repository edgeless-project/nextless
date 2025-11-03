// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

pub static ID: super::DialectId = super::DialectId("PHYSICAL_OVERLAY");

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
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
impl super::Interaction for PhyscialOverlayInteraction {
    fn as_physical(&self) -> Option<&dyn super::PhysicalInteraction> {
        return Some(self);
    }
}

impl super::PhysicalInteraction for PhyscialOverlayInteraction {
    fn relevant_nodes(&self) -> Vec<edgeless_api::function_instance::NodeId> {
        let mut nodes = std::collections::BTreeSet::new();

        for s in &self.sources {
            nodes.insert(s.instance.node_id.clone());
        }

        match &self.destination {
            DestinationMapping::Unicast(physical_port_id) => {
                nodes.insert(physical_port_id.instance.node_id.clone());
            }
            DestinationMapping::Anycast(physical_port_ids) => {
                for port in physical_port_ids {
                    nodes.insert(port.instance.node_id.clone());
                }
            }
            DestinationMapping::Multicast(physical_port_ids) => {
                for port in physical_port_ids {
                    nodes.insert(port.instance.node_id.clone());
                }
            }
        }

        nodes.into_iter().collect()
    }
}

impl super::InteractionPortUtils<crate::ir::interaction::PhysicalPortId> for PhysicalOverlayDialect {
    fn ports_to_interaction(
        &self,
        srcs: Vec<(crate::ir::interaction::PhysicalPortId, super::super::SourcePortMapping)>,
        _dests: Vec<(crate::ir::interaction::PhysicalPortId, super::super::DestiantionPortMapping)>,
    ) -> Vec<super::super::InteractionMapping> {
        let mut collector = std::collections::BTreeMap::<DestinationMapping, Vec<crate::ir::interaction::PhysicalPortId>>::new();

        for (src_port_id, src_spec) in srcs {
            let any_mapping = src_spec.mapping.as_ref() as &dyn std::any::Any;
            let maybe_overlay_mapping = any_mapping.downcast_ref::<PhysicalOverlaySourcePort>();
            let overlay_mapping = maybe_overlay_mapping.unwrap();
            collector.entry(overlay_mapping.destination.clone()).or_default().push(src_port_id)
        }

        collector
            .into_iter()
            .map(|(destination, sources)| super::super::InteractionMapping {
                mapping: Box::new(PhyscialOverlayInteraction { sources, destination }),
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
        Vec<(crate::ir::interaction::PhysicalPortId, super::super::SourcePortMapping)>,
        Vec<(crate::ir::interaction::PhysicalPortId, super::super::DestiantionPortMapping)>,
    ) {
        let mut srcs = Vec::new();
        let mut dests = Vec::new();

        let any_mapping = interaction.mapping.as_ref() as &dyn std::any::Any;
        let maybe_overlay_mapping = any_mapping.downcast_ref::<PhyscialOverlayInteraction>();
        let overlay_mapping = maybe_overlay_mapping.unwrap();

        for src_port_id in overlay_mapping.sources.clone() {
            srcs.push((
                src_port_id,
                super::super::SourcePortMapping {
                    mapping: Box::new(PhysicalOverlaySourcePort {
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
                    PhysicalOverlayDestinationPort {
                        sources: overlay_mapping.sources.clone(),
                    },
                ));
            }
            DestinationMapping::Anycast(dest_port_ids) => {
                for dest_port_id in dest_port_ids {
                    dests.push((
                        dest_port_id.clone(),
                        PhysicalOverlayDestinationPort {
                            sources: overlay_mapping.sources.clone(),
                        },
                    ));
                }
            }
            DestinationMapping::Multicast(dest_port_ids) => {
                for dest_port_id in dest_port_ids {
                    dests.push((
                        dest_port_id.clone(),
                        PhysicalOverlayDestinationPort {
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

impl super::InteractionDialect for PhysicalOverlayDialect {
    fn id(&self) -> super::DialectId {
        ID
    }

    fn provides_transformations_to(&self) -> &'static [super::DialectId] {
        static TRANSFORMATIONS: [super::DialectId; 1] = [super::ip_multicast::ID];
        &TRANSFORMATIONS
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
        return None;
    }

    fn physical_utils(&self) -> Option<&dyn super::InteractionPortUtils<crate::ir::interaction::PhysicalPortId>> {
        return Some(self);
    }

    fn link_config(
        &self,
        _mapping: &crate::ir::interaction::InteractionMapping,
        _nodes: &std::collections::HashMap<uuid::Uuid, &dyn crate::ir::Node>,
    ) -> crate::ir::link::WorkflowLink {
        todo!()
    }
}
