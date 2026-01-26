// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

use crate::ir::interaction::dialect::{AsConcreteInteraction, AsConcreteSourcePort};

pub static ID: super::DialectId = super::DialectId("PHYSICAL_OVERLAY");

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum PhysicalOverlayConstraint {
    Cluster(uuid::Uuid),
}

impl super::DialectConstraint for PhysicalOverlayConstraint {
    fn as_container(self) -> super::DialectConstraintContainer {
        super::DialectConstraintContainer {
            dialect: ID,
            constraint: Box::new(self),
        }
    }
}

pub struct PhysicalOverlayDialect {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhysicalOverlaySourcePort {
    pub destination: DestinationMapping,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysicalOverlayDestinationPort {
    pub sources: std::collections::BTreeSet<crate::ir::interaction::PhysicalPortId>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhyscialOverlayInteraction {
    pub destination: DestinationMapping,
    pub sources: std::collections::BTreeSet<crate::ir::interaction::PhysicalPortId>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum DestinationMapping {
    Unicast(crate::ir::interaction::PhysicalPortId),
    Anycast(std::collections::BTreeSet<crate::ir::interaction::PhysicalPortId>),
    Multicast(std::collections::BTreeSet<crate::ir::interaction::PhysicalPortId>),
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
    ) -> Result<Vec<super::super::InteractionMapping>, crate::ir::interaction::InteractionError> {
        let mut collector =
            std::collections::BTreeMap::<DestinationMapping, std::collections::BTreeSet<crate::ir::interaction::PhysicalPortId>>::new();
        let mut constraints: Option<std::collections::BTreeSet<super::DialectConstraintContainer>> = None;

        for (src_port_id, src_spec) in srcs {
            let overlay_mapping = PhysicalOverlaySourcePort::as_concrete(src_spec.mapping.as_ref())?;

            // TODO: This check needs to be done per link, not assume the same ports for all links.
            if let Some(constraints) = &mut constraints {
                if *constraints != src_spec.dialect_type.constraints {
                    tracing::warn!("Merging Ports with different constraints");
                }
            } else {
                constraints = Some(src_spec.dialect_type.constraints.clone())
            }

            collector.entry(overlay_mapping.destination.clone()).or_default().insert(src_port_id);
        }

        Ok(collector
            .into_iter()
            .map(|(destination, sources)| super::super::InteractionMapping {
                mapping: Box::new(PhyscialOverlayInteraction { sources, destination }),
                dialect_type: super::DialectDescriptor {
                    base_type: ID,
                    constraints: constraints.clone().unwrap_or_default(),
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
        let mut srcs = std::collections::BTreeMap::new();
        let mut dests = std::collections::BTreeMap::new();

        let overlay_mapping = PhyscialOverlayInteraction::as_concrete(interaction.mapping.as_ref())?;

        for src_port_id in overlay_mapping.sources.clone() {
            let old = srcs.insert(
                src_port_id,
                super::super::SourcePortMapping {
                    mapping: Box::new(PhysicalOverlaySourcePort {
                        destination: overlay_mapping.destination.clone(),
                    }),
                    dialect_type: interaction.dialect_type.clone(),
                },
            );

            if old.is_some() {
                tracing::warn!("Duplicate entry for source port. Old Value replaced.");
            }
        }

        match &overlay_mapping.destination {
            DestinationMapping::Unicast(dest_port_id) => {
                dests
                    .entry(dest_port_id.clone())
                    .or_insert_with(|| PhysicalOverlayDestinationPort { sources: Default::default() })
                    .sources
                    .extend(overlay_mapping.sources.clone());
            }
            DestinationMapping::Anycast(dest_port_ids) => {
                for dest_port_id in dest_port_ids {
                    dests
                        .entry(dest_port_id.clone())
                        .or_insert_with(|| PhysicalOverlayDestinationPort { sources: Default::default() })
                        .sources
                        .extend(overlay_mapping.sources.clone());
                }
            }
            DestinationMapping::Multicast(dest_port_ids) => {
                for dest_port_id in dest_port_ids {
                    dests
                        .entry(dest_port_id.clone())
                        .or_insert_with(|| PhysicalOverlayDestinationPort { sources: Default::default() })
                        .sources
                        .extend(overlay_mapping.sources.clone());
                }
            }
        }

        let dests = dests
            .into_iter()
            .map(|(id, mapping)| {
                (
                    id,
                    super::super::DestiantionPortMapping {
                        dialect_type: interaction.dialect_type.clone(),
                        mapping: Box::new(mapping),
                    },
                )
            })
            .collect();

        Ok((srcs.into_iter().collect(), dests))
    }
}

impl super::InteractionDialect for PhysicalOverlayDialect {
    fn id(&self) -> super::DialectId {
        ID
    }

    fn provides_transformations_to(&self) -> &'static [super::DialectId] {
        &[]
    }

    fn provides_transformations_from(&self) -> &'static [super::DialectId] {
        &[]
    }

    fn plan_translation_to(
        &self,
        src: &super::super::InteractionMapping,
        dst: &super::DialectDescriptor,
    ) -> Result<(super::DialectDescriptor, u64), crate::ir::interaction::InteractionError> {
        Err(crate::ir::interaction::InteractionError::UnsupportedTranslation(
            src.dialect_type.clone(),
            dst.clone(),
        ))
    }

    fn execute_transformation_to(
        &self,
        src: &crate::ir::interaction::InteractionMapping,
        dest: &super::DialectDescriptor,
    ) -> Result<Vec<crate::ir::interaction::InteractionMapping>, crate::ir::interaction::InteractionError> {
        Err(crate::ir::interaction::InteractionError::UnsupportedTranslation(
            src.dialect_type.clone(),
            dest.clone(),
        ))
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
    ) -> crate::ir::interaction::LinkConfigurationResult {
        crate::ir::interaction::LinkConfigurationResult::NoConfig
    }
}

#[cfg(test)]
mod port_interaction_test {
    use crate::ir::interaction::{
        dialect::{AsConcreteDestinationPort, AsConcreteInteraction, AsConcreteSourcePort, InteractionPortUtils},
        PhysicalPortId,
    };

    #[test]
    fn single_source_unicast_target() {
        let input_source_id = example_port_id("output1");
        let input_destination_id = example_port_id("input1");

        let input_source_port_mapping = crate::ir::interaction::SourcePortMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(super::PhysicalOverlaySourcePort {
                destination: super::DestinationMapping::Unicast(input_destination_id.clone()),
            }),
        };

        let expected_destination_port_mapping = crate::ir::interaction::DestiantionPortMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(super::PhysicalOverlayDestinationPort {
                sources: std::collections::BTreeSet::from([input_source_id.clone()]),
            }),
        };

        let expected_destination_ports = vec![(input_destination_id.clone(), vec![expected_destination_port_mapping])];

        let expected_interaction_mapping = crate::ir::interaction::InteractionMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(super::PhyscialOverlayInteraction {
                destination: super::DestinationMapping::Unicast(input_destination_id.clone()),
                sources: std::collections::BTreeSet::from([input_source_id.clone()]),
            }),
        };

        let input_source_ports: Vec<(crate::ir::interaction::PhysicalPortId, crate::ir::interaction::SourcePortMapping)> =
            vec![(input_source_id, input_source_port_mapping)];

        port_interaction_roundtrip_test(
            &input_source_ports,
            &[],
            &[expected_interaction_mapping],
            &input_source_ports,
            &expected_destination_ports,
        );
    }

    #[test]
    fn single_source_anycast_target() {
        let input_source_id = example_port_id("output1");
        let input_destination_id1 = example_port_id("input1");
        let input_destination_id2 = example_port_id("input2");

        let input_source_port_mapping = crate::ir::interaction::SourcePortMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(super::PhysicalOverlaySourcePort {
                destination: super::DestinationMapping::Anycast(std::collections::BTreeSet::from([
                    input_destination_id1.clone(),
                    input_destination_id2.clone(),
                ])),
            }),
        };

        let expected_destination_port_mapping = crate::ir::interaction::DestiantionPortMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(super::PhysicalOverlayDestinationPort {
                sources: std::collections::BTreeSet::from([input_source_id.clone()]),
            }),
        };

        let expected_destination_ports = vec![
            (input_destination_id1.clone(), vec![expected_destination_port_mapping.clone()]),
            (input_destination_id2.clone(), vec![expected_destination_port_mapping]),
        ];

        let expected_interaction_mapping = crate::ir::interaction::InteractionMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(super::PhyscialOverlayInteraction {
                destination: super::DestinationMapping::Anycast(std::collections::BTreeSet::from([
                    input_destination_id1.clone(),
                    input_destination_id2.clone(),
                ])),
                sources: std::collections::BTreeSet::from([input_source_id.clone()]),
            }),
        };

        let input_source_ports: Vec<(crate::ir::interaction::PhysicalPortId, crate::ir::interaction::SourcePortMapping)> =
            vec![(input_source_id, input_source_port_mapping)];

        port_interaction_roundtrip_test(
            &input_source_ports,
            &[],
            &[expected_interaction_mapping],
            &input_source_ports,
            &expected_destination_ports,
        );
    }

    #[test]
    fn single_source_multicast_target() {
        let input_source_id = example_port_id("output1");
        let input_destination_id1 = example_port_id("input1");
        let input_destination_id2 = example_port_id("input2");

        let input_source_port_mapping = crate::ir::interaction::SourcePortMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(super::PhysicalOverlaySourcePort {
                destination: super::DestinationMapping::Multicast(std::collections::BTreeSet::from([
                    input_destination_id1.clone(),
                    input_destination_id2.clone(),
                ])),
            }),
        };

        let expected_destination_port_mapping = crate::ir::interaction::DestiantionPortMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(super::PhysicalOverlayDestinationPort {
                sources: std::collections::BTreeSet::from([input_source_id.clone()]),
            }),
        };

        let expected_destination_ports = vec![
            (input_destination_id1.clone(), vec![expected_destination_port_mapping.clone()]),
            (input_destination_id2.clone(), vec![expected_destination_port_mapping]),
        ];

        let expected_interaction_mapping = crate::ir::interaction::InteractionMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(super::PhyscialOverlayInteraction {
                destination: super::DestinationMapping::Multicast(std::collections::BTreeSet::from([
                    input_destination_id1.clone(),
                    input_destination_id2.clone(),
                ])),
                sources: std::collections::BTreeSet::from([input_source_id.clone()]),
            }),
        };

        let input_source_ports: Vec<(crate::ir::interaction::PhysicalPortId, crate::ir::interaction::SourcePortMapping)> =
            vec![(input_source_id, input_source_port_mapping)];

        port_interaction_roundtrip_test(
            &input_source_ports,
            &[],
            &[expected_interaction_mapping],
            &input_source_ports,
            &expected_destination_ports,
        );
    }

    #[test]
    fn two_sources_different_unicast_target() {
        let input_source_id1 = example_port_id("output1");
        let input_source_id2 = example_port_id("output2");
        let input_destination_id1 = example_port_id("input1");
        let input_destination_id2 = example_port_id("input2");

        let input_source_port_mapping1 = crate::ir::interaction::SourcePortMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(super::PhysicalOverlaySourcePort {
                destination: super::DestinationMapping::Unicast(input_destination_id1.clone()),
            }),
        };

        let input_source_port_mapping2 = crate::ir::interaction::SourcePortMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(super::PhysicalOverlaySourcePort {
                destination: super::DestinationMapping::Unicast(input_destination_id2.clone()),
            }),
        };

        let expected_destination_port_mapping1 = crate::ir::interaction::DestiantionPortMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(super::PhysicalOverlayDestinationPort {
                sources: std::collections::BTreeSet::from([input_source_id1.clone()]),
            }),
        };

        let expected_destination_port_mapping2 = crate::ir::interaction::DestiantionPortMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(super::PhysicalOverlayDestinationPort {
                sources: std::collections::BTreeSet::from([input_source_id2.clone()]),
            }),
        };

        let expected_destination_ports = vec![
            (input_destination_id1.clone(), vec![expected_destination_port_mapping1]),
            (input_destination_id2.clone(), vec![expected_destination_port_mapping2]),
        ];

        let expected_interaction_mapping1 = super::PhyscialOverlayInteraction {
            destination: super::DestinationMapping::Unicast(input_destination_id1.clone()),
            sources: std::collections::BTreeSet::from([input_source_id1.clone()]),
        };

        let expected_interaction_mapping2 = super::PhyscialOverlayInteraction {
            destination: super::DestinationMapping::Unicast(input_destination_id2.clone()),
            sources: std::collections::BTreeSet::from([input_source_id2.clone()]),
        };

        let expected_interaction1 = crate::ir::interaction::InteractionMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(expected_interaction_mapping1),
        };

        let expected_interaction2 = crate::ir::interaction::InteractionMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(expected_interaction_mapping2),
        };

        let expected_interactions = vec![expected_interaction1.clone(), expected_interaction2.clone()];

        let input_source_ports = vec![
            (input_source_id1, input_source_port_mapping1.clone()),
            (input_source_id2, input_source_port_mapping2.clone()),
        ];

        port_interaction_roundtrip_test(
            &input_source_ports,
            &[],
            &expected_interactions,
            &input_source_ports,
            &expected_destination_ports,
        );
    }

    #[test]
    fn two_sources_different_anycast_targets() {
        let input_source_id1 = example_port_id("output1");
        let input_source_id2 = example_port_id("output2");
        let input_destination_id1 = example_port_id("input1");
        let input_destination_id2 = example_port_id("input2");
        let input_destination_id3 = example_port_id("input1");
        let input_destination_id4 = example_port_id("input2");

        let input_source_port_mapping1 = crate::ir::interaction::SourcePortMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(super::PhysicalOverlaySourcePort {
                destination: super::DestinationMapping::Anycast(std::collections::BTreeSet::from([
                    input_destination_id1.clone(),
                    input_destination_id2.clone(),
                ])),
            }),
        };

        let input_source_port_mapping2 = crate::ir::interaction::SourcePortMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(super::PhysicalOverlaySourcePort {
                destination: super::DestinationMapping::Anycast(std::collections::BTreeSet::from([
                    input_destination_id3.clone(),
                    input_destination_id4.clone(),
                ])),
            }),
        };

        let expected_destination_port_mapping1 = crate::ir::interaction::DestiantionPortMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(super::PhysicalOverlayDestinationPort {
                sources: std::collections::BTreeSet::from([input_source_id1.clone()]),
            }),
        };

        let expected_destination_port_mapping2 = crate::ir::interaction::DestiantionPortMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(super::PhysicalOverlayDestinationPort {
                sources: std::collections::BTreeSet::from([input_source_id1.clone()]),
            }),
        };

        let expected_destination_port_mapping3 = crate::ir::interaction::DestiantionPortMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(super::PhysicalOverlayDestinationPort {
                sources: std::collections::BTreeSet::from([input_source_id2.clone()]),
            }),
        };

        let expected_destination_port_mapping4 = crate::ir::interaction::DestiantionPortMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(super::PhysicalOverlayDestinationPort {
                sources: std::collections::BTreeSet::from([input_source_id2.clone()]),
            }),
        };

        let expected_destination_ports = vec![
            (input_destination_id1.clone(), vec![expected_destination_port_mapping1]),
            (input_destination_id2.clone(), vec![expected_destination_port_mapping2]),
            (input_destination_id3.clone(), vec![expected_destination_port_mapping3]),
            (input_destination_id4.clone(), vec![expected_destination_port_mapping4]),
        ];

        let expected_interaction_mapping1 = super::PhyscialOverlayInteraction {
            destination: super::DestinationMapping::Anycast(std::collections::BTreeSet::from([
                input_destination_id1.clone(),
                input_destination_id2.clone(),
            ])),
            sources: std::collections::BTreeSet::from([input_source_id1.clone()]),
        };

        let expected_interaction_mapping2 = super::PhyscialOverlayInteraction {
            destination: super::DestinationMapping::Anycast(std::collections::BTreeSet::from([
                input_destination_id3.clone(),
                input_destination_id4.clone(),
            ])),
            sources: std::collections::BTreeSet::from([input_source_id2.clone()]),
        };

        let expected_interaction1 = crate::ir::interaction::InteractionMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(expected_interaction_mapping1),
        };

        let expected_interaction2 = crate::ir::interaction::InteractionMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(expected_interaction_mapping2),
        };

        let expected_interactions = vec![expected_interaction1.clone(), expected_interaction2.clone()];

        let input_source_ports = vec![
            (input_source_id1, input_source_port_mapping1.clone()),
            (input_source_id2, input_source_port_mapping2.clone()),
        ];

        port_interaction_roundtrip_test(
            &input_source_ports,
            &[],
            &expected_interactions,
            &input_source_ports,
            &expected_destination_ports,
        );
    }

    #[test]
    fn two_sources_different_multicast_targets() {
        let input_source_id1 = example_port_id("output1");
        let input_source_id2 = example_port_id("output2");
        let input_destination_id1 = example_port_id("input1");
        let input_destination_id2 = example_port_id("input2");
        let input_destination_id3 = example_port_id("input1");
        let input_destination_id4 = example_port_id("input2");

        let input_source_port_mapping1 = crate::ir::interaction::SourcePortMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(super::PhysicalOverlaySourcePort {
                destination: super::DestinationMapping::Multicast(std::collections::BTreeSet::from([
                    input_destination_id1.clone(),
                    input_destination_id2.clone(),
                ])),
            }),
        };

        let input_source_port_mapping2 = crate::ir::interaction::SourcePortMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(super::PhysicalOverlaySourcePort {
                destination: super::DestinationMapping::Multicast(std::collections::BTreeSet::from([
                    input_destination_id3.clone(),
                    input_destination_id4.clone(),
                ])),
            }),
        };

        let expected_destination_port_mapping1 = crate::ir::interaction::DestiantionPortMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(super::PhysicalOverlayDestinationPort {
                sources: std::collections::BTreeSet::from([input_source_id1.clone()]),
            }),
        };

        let expected_destination_port_mapping2 = crate::ir::interaction::DestiantionPortMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(super::PhysicalOverlayDestinationPort {
                sources: std::collections::BTreeSet::from([input_source_id1.clone()]),
            }),
        };

        let expected_destination_port_mapping3 = crate::ir::interaction::DestiantionPortMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(super::PhysicalOverlayDestinationPort {
                sources: std::collections::BTreeSet::from([input_source_id2.clone()]),
            }),
        };

        let expected_destination_port_mapping4 = crate::ir::interaction::DestiantionPortMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(super::PhysicalOverlayDestinationPort {
                sources: std::collections::BTreeSet::from([input_source_id2.clone()]),
            }),
        };

        let expected_destination_ports = vec![
            (input_destination_id1.clone(), vec![expected_destination_port_mapping1]),
            (input_destination_id2.clone(), vec![expected_destination_port_mapping2]),
            (input_destination_id3.clone(), vec![expected_destination_port_mapping3]),
            (input_destination_id4.clone(), vec![expected_destination_port_mapping4]),
        ];

        let expected_interaction_mapping1 = super::PhyscialOverlayInteraction {
            destination: super::DestinationMapping::Multicast(std::collections::BTreeSet::from([
                input_destination_id1.clone(),
                input_destination_id2.clone(),
            ])),
            sources: std::collections::BTreeSet::from([input_source_id1.clone()]),
        };

        let expected_interaction_mapping2 = super::PhyscialOverlayInteraction {
            destination: super::DestinationMapping::Multicast(std::collections::BTreeSet::from([
                input_destination_id3.clone(),
                input_destination_id4.clone(),
            ])),
            sources: std::collections::BTreeSet::from([input_source_id2.clone()]),
        };

        let expected_interaction1 = crate::ir::interaction::InteractionMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(expected_interaction_mapping1),
        };

        let expected_interaction2 = crate::ir::interaction::InteractionMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(expected_interaction_mapping2),
        };

        let expected_interactions = vec![expected_interaction1.clone(), expected_interaction2.clone()];

        let input_source_ports = vec![
            (input_source_id1, input_source_port_mapping1.clone()),
            (input_source_id2, input_source_port_mapping2.clone()),
        ];

        port_interaction_roundtrip_test(
            &input_source_ports,
            &[],
            &expected_interactions,
            &input_source_ports,
            &expected_destination_ports,
        );
    }

    #[test]
    fn two_sources_shared_unicast_target() {
        let input_source_id1 = example_port_id("output1");
        let input_source_id2 = example_port_id("output2");
        let input_destination_id = example_port_id("input1");

        let input_source_port_mapping = crate::ir::interaction::SourcePortMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(super::PhysicalOverlaySourcePort {
                destination: super::DestinationMapping::Unicast(input_destination_id.clone()),
            }),
        };

        let expected_interaction_mapping_destinations = super::DestinationMapping::Unicast(input_destination_id.clone());
        let expected_interaction_mapping_sources = std::collections::BTreeSet::from([input_source_id1.clone(), input_source_id2.clone()]);

        let expected_interaction = crate::ir::interaction::InteractionMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(super::PhyscialOverlayInteraction {
                destination: expected_interaction_mapping_destinations,
                sources: expected_interaction_mapping_sources.clone(),
            }),
        };

        let input_source_ports = vec![
            (input_source_id1, input_source_port_mapping.clone()),
            (input_source_id2, input_source_port_mapping.clone()),
        ];

        let expected_destination_port = crate::ir::interaction::DestiantionPortMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(super::PhysicalOverlayDestinationPort {
                sources: expected_interaction_mapping_sources.clone(),
            }),
        };

        port_interaction_roundtrip_test(
            &input_source_ports,
            &[],
            &[expected_interaction],
            &input_source_ports,
            &[(input_destination_id.clone(), vec![expected_destination_port])],
        );
    }

    #[test]
    fn two_sources_shared_anycast_target() {
        let input_source_id1 = example_port_id("output1");
        let input_source_id2 = example_port_id("output2");
        let input_destination_id1 = example_port_id("input1");
        let input_destination_id2 = example_port_id("input2");

        let input_source_port_mapping = crate::ir::interaction::SourcePortMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(super::PhysicalOverlaySourcePort {
                destination: super::DestinationMapping::Anycast(std::collections::BTreeSet::from([
                    input_destination_id1.clone(),
                    input_destination_id2.clone(),
                ])),
            }),
        };

        let expected_interaction_mapping_destinations = super::DestinationMapping::Anycast(std::collections::BTreeSet::from([
            input_destination_id1.clone(),
            input_destination_id2.clone(),
        ]));
        let expected_interaction_mapping_sources = std::collections::BTreeSet::from([input_source_id1.clone(), input_source_id2.clone()]);

        let expected_interaction = crate::ir::interaction::InteractionMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(super::PhyscialOverlayInteraction {
                destination: expected_interaction_mapping_destinations,
                sources: expected_interaction_mapping_sources.clone(),
            }),
        };

        let input_source_ports = vec![
            (input_source_id1, input_source_port_mapping.clone()),
            (input_source_id2, input_source_port_mapping.clone()),
        ];

        let expected_destination_port1 = crate::ir::interaction::DestiantionPortMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(super::PhysicalOverlayDestinationPort {
                sources: expected_interaction_mapping_sources.clone(),
            }),
        };

        let expected_destination_port2 = crate::ir::interaction::DestiantionPortMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(super::PhysicalOverlayDestinationPort {
                sources: expected_interaction_mapping_sources.clone(),
            }),
        };

        let expected_destination_ports = vec![
            (input_destination_id1.clone(), vec![expected_destination_port1]),
            (input_destination_id2.clone(), vec![expected_destination_port2]),
        ];

        port_interaction_roundtrip_test(
            &input_source_ports,
            &[],
            &[expected_interaction],
            &input_source_ports,
            &expected_destination_ports,
        );
    }

    #[test]
    fn two_sources_shared_multicast_target() {
        let input_source_id1 = example_port_id("output1");
        let input_source_id2 = example_port_id("output2");
        let input_destination_id1 = example_port_id("input1");
        let input_destination_id2 = example_port_id("input2");

        let input_source_port_mapping = crate::ir::interaction::SourcePortMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(super::PhysicalOverlaySourcePort {
                destination: super::DestinationMapping::Multicast(std::collections::BTreeSet::from([
                    input_destination_id1.clone(),
                    input_destination_id2.clone(),
                ])),
            }),
        };

        let expected_interaction_mapping_destinations = super::DestinationMapping::Multicast(std::collections::BTreeSet::from([
            input_destination_id1.clone(),
            input_destination_id2.clone(),
        ]));
        let expected_interaction_mapping_sources = std::collections::BTreeSet::from([input_source_id1.clone(), input_source_id2.clone()]);

        let expected_interaction = crate::ir::interaction::InteractionMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(super::PhyscialOverlayInteraction {
                destination: expected_interaction_mapping_destinations,
                sources: expected_interaction_mapping_sources.clone(),
            }),
        };

        let input_source_ports = vec![
            (input_source_id1, input_source_port_mapping.clone()),
            (input_source_id2, input_source_port_mapping.clone()),
        ];

        let expected_destination_port1 = crate::ir::interaction::DestiantionPortMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(super::PhysicalOverlayDestinationPort {
                sources: expected_interaction_mapping_sources.clone(),
            }),
        };

        let expected_destination_port2 = crate::ir::interaction::DestiantionPortMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(super::PhysicalOverlayDestinationPort {
                sources: expected_interaction_mapping_sources.clone(),
            }),
        };

        let expected_destination_ports = vec![
            (input_destination_id1.clone(), vec![expected_destination_port1]),
            (input_destination_id2.clone(), vec![expected_destination_port2]),
        ];

        port_interaction_roundtrip_test(
            &input_source_ports,
            &[],
            &[expected_interaction],
            &input_source_ports,
            &expected_destination_ports,
        );
    }

    #[test]
    fn two_sources_overlapping_anycast_targets() {
        let input_source_id1 = example_port_id("output1");
        let input_source_id2 = example_port_id("output2");
        let input_destination_id1 = example_port_id("input1");
        let input_destination_id2 = example_port_id("input2");
        let input_destination_id3 = example_port_id("input3");

        let input_source_port_mapping1 = crate::ir::interaction::SourcePortMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(super::PhysicalOverlaySourcePort {
                destination: super::DestinationMapping::Anycast(std::collections::BTreeSet::from([
                    input_destination_id1.clone(),
                    input_destination_id2.clone(),
                ])),
            }),
        };

        let input_source_port_mapping2 = crate::ir::interaction::SourcePortMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(super::PhysicalOverlaySourcePort {
                destination: super::DestinationMapping::Anycast(std::collections::BTreeSet::from([
                    input_destination_id2.clone(),
                    input_destination_id3.clone(),
                ])),
            }),
        };

        let expected_interaction_mapping_destinations1 = super::DestinationMapping::Anycast(std::collections::BTreeSet::from([
            input_destination_id1.clone(),
            input_destination_id2.clone(),
        ]));
        let expected_interaction_mapping_destinations2 = super::DestinationMapping::Anycast(std::collections::BTreeSet::from([
            input_destination_id2.clone(),
            input_destination_id3.clone(),
        ]));
        let expected_interaction_mapping_sources1 = std::collections::BTreeSet::from([input_source_id1.clone()]);
        let expected_interaction_mapping_sources3 = std::collections::BTreeSet::from([input_source_id2.clone()]);

        let expected_interaction1 = crate::ir::interaction::InteractionMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(super::PhyscialOverlayInteraction {
                destination: expected_interaction_mapping_destinations1,
                sources: expected_interaction_mapping_sources1.clone(),
            }),
        };

        let expected_interaction2 = crate::ir::interaction::InteractionMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(super::PhyscialOverlayInteraction {
                destination: expected_interaction_mapping_destinations2,
                sources: expected_interaction_mapping_sources3.clone(),
            }),
        };

        let input_source_ports = vec![
            (input_source_id1, input_source_port_mapping1.clone()),
            (input_source_id2, input_source_port_mapping2.clone()),
        ];

        let expected_destination_port1 = crate::ir::interaction::DestiantionPortMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(super::PhysicalOverlayDestinationPort {
                sources: expected_interaction_mapping_sources1.clone(),
            }),
        };

        let expected_destination_port2_1 = crate::ir::interaction::DestiantionPortMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(super::PhysicalOverlayDestinationPort {
                sources: expected_interaction_mapping_sources1,
            }),
        };

        let expected_destination_port2_2 = crate::ir::interaction::DestiantionPortMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(super::PhysicalOverlayDestinationPort {
                sources: expected_interaction_mapping_sources3.clone(),
            }),
        };

        let expected_destination_port3 = crate::ir::interaction::DestiantionPortMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(super::PhysicalOverlayDestinationPort {
                sources: expected_interaction_mapping_sources3.clone(),
            }),
        };

        let expected_destination_ports = vec![
            (input_destination_id1.clone(), vec![expected_destination_port1]),
            (
                input_destination_id2.clone(),
                vec![expected_destination_port2_1, expected_destination_port2_2],
            ),
            (input_destination_id3.clone(), vec![expected_destination_port3]),
        ];

        port_interaction_roundtrip_test(
            &input_source_ports,
            &[],
            &[expected_interaction1, expected_interaction2],
            &input_source_ports,
            &expected_destination_ports,
        );
    }

    #[test]
    fn two_sources_overlapping_multicast_targets() {
        let input_source_id1 = example_port_id("output1");
        let input_source_id2 = example_port_id("output2");
        let input_destination_id1 = example_port_id("input1");
        let input_destination_id2 = example_port_id("input2");
        let input_destination_id3 = example_port_id("input3");

        let input_source_port_mapping1 = crate::ir::interaction::SourcePortMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(super::PhysicalOverlaySourcePort {
                destination: super::DestinationMapping::Multicast(std::collections::BTreeSet::from([
                    input_destination_id1.clone(),
                    input_destination_id2.clone(),
                ])),
            }),
        };

        let input_source_port_mapping2 = crate::ir::interaction::SourcePortMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(super::PhysicalOverlaySourcePort {
                destination: super::DestinationMapping::Multicast(std::collections::BTreeSet::from([
                    input_destination_id2.clone(),
                    input_destination_id3.clone(),
                ])),
            }),
        };

        let expected_interaction_mapping_destinations1 = super::DestinationMapping::Multicast(std::collections::BTreeSet::from([
            input_destination_id1.clone(),
            input_destination_id2.clone(),
        ]));
        let expected_interaction_mapping_destinations2 = super::DestinationMapping::Multicast(std::collections::BTreeSet::from([
            input_destination_id2.clone(),
            input_destination_id3.clone(),
        ]));
        let expected_interaction_mapping_sources1 = std::collections::BTreeSet::from([input_source_id1.clone()]);
        let expected_interaction_mapping_sources3 = std::collections::BTreeSet::from([input_source_id2.clone()]);

        let expected_interaction1 = crate::ir::interaction::InteractionMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(super::PhyscialOverlayInteraction {
                destination: expected_interaction_mapping_destinations1,
                sources: expected_interaction_mapping_sources1.clone(),
            }),
        };

        let expected_interaction2 = crate::ir::interaction::InteractionMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(super::PhyscialOverlayInteraction {
                destination: expected_interaction_mapping_destinations2,
                sources: expected_interaction_mapping_sources3.clone(),
            }),
        };

        let input_source_ports = vec![
            (input_source_id1, input_source_port_mapping1.clone()),
            (input_source_id2, input_source_port_mapping2.clone()),
        ];

        let expected_destination_port1 = crate::ir::interaction::DestiantionPortMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(super::PhysicalOverlayDestinationPort {
                sources: expected_interaction_mapping_sources1.clone(),
            }),
        };

        let expected_destination_port2_1 = crate::ir::interaction::DestiantionPortMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(super::PhysicalOverlayDestinationPort {
                sources: expected_interaction_mapping_sources1,
            }),
        };

        let expected_destination_port2_2 = crate::ir::interaction::DestiantionPortMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(super::PhysicalOverlayDestinationPort {
                sources: expected_interaction_mapping_sources3.clone(),
            }),
        };

        let expected_destination_port3 = crate::ir::interaction::DestiantionPortMapping {
            dialect_type: dialect_type(),
            mapping: Box::new(super::PhysicalOverlayDestinationPort {
                sources: expected_interaction_mapping_sources3.clone(),
            }),
        };

        let expected_destination_ports = vec![
            (input_destination_id1.clone(), vec![expected_destination_port1]),
            (
                input_destination_id2.clone(),
                vec![expected_destination_port2_1, expected_destination_port2_2],
            ),
            (input_destination_id3.clone(), vec![expected_destination_port3]),
        ];

        port_interaction_roundtrip_test(
            &input_source_ports,
            &[],
            &[expected_interaction1, expected_interaction2],
            &input_source_ports,
            &expected_destination_ports,
        );
    }

    fn port_interaction_roundtrip_test(
        input_source_ports: &[(crate::ir::interaction::PhysicalPortId, crate::ir::interaction::SourcePortMapping)],
        input_destination_ports: &[(crate::ir::interaction::PhysicalPortId, crate::ir::interaction::DestiantionPortMapping)],
        expected_interactions: &[crate::ir::interaction::InteractionMapping],
        expected_source_ports: &[(PhysicalPortId, crate::ir::interaction::SourcePortMapping)],
        expected_destination_ports: &[(PhysicalPortId, Vec<crate::ir::interaction::DestiantionPortMapping>)],
    ) {
        let dialect_type = dialect_type();

        let result_interactions = super::PhysicalOverlayDialect {}
            .ports_to_interaction(input_source_ports.to_vec(), input_destination_ports.to_vec())
            .unwrap();

        let mut parsed_result_interactions = std::collections::BTreeSet::new();
        let mut parsed_expected_interactions = std::collections::BTreeSet::new();

        for result_interaction in &result_interactions {
            assert_eq!(result_interaction.dialect_type, dialect_type);
            assert!(parsed_result_interactions.insert(super::PhyscialOverlayInteraction::as_concrete(result_interaction.mapping.as_ref()).unwrap()));
        }

        for expected_interaction in expected_interactions {
            assert_eq!(expected_interaction.dialect_type, dialect_type);
            assert!(
                parsed_expected_interactions.insert(super::PhyscialOverlayInteraction::as_concrete(expected_interaction.mapping.as_ref()).unwrap())
            );
        }

        assert_eq!(parsed_expected_interactions, parsed_result_interactions);

        let mut collected_result_source_ports = std::collections::BTreeMap::new();
        let mut collected_result_destination_ports: std::collections::BTreeMap<
            PhysicalPortId,
            std::collections::BTreeSet<super::PhysicalOverlayDestinationPort>,
        > = std::collections::BTreeMap::new();
        let mut collected_expected_source_ports = std::collections::BTreeMap::new();
        let mut collected_expected_destination_ports: std::collections::BTreeMap<
            PhysicalPortId,
            std::collections::BTreeSet<super::PhysicalOverlayDestinationPort>,
        > = std::collections::BTreeMap::new();

        for interaction in &result_interactions {
            let (source_ports, destination_ports) = super::PhysicalOverlayDialect {}.interaction_to_ports(interaction.clone()).unwrap();

            for (source_port_id, source_port_mapping) in source_ports {
                assert_eq!(source_port_mapping.dialect_type, dialect_type);
                assert!(collected_result_source_ports
                    .insert(
                        source_port_id.clone(),
                        super::PhysicalOverlaySourcePort::as_concrete(source_port_mapping.mapping.as_ref())
                            .unwrap()
                            .clone()
                    )
                    .is_none())
            }

            for (destination_port_id, destination_port_mapping) in destination_ports {
                assert_eq!(destination_port_mapping.dialect_type, dialect_type);
                collected_result_destination_ports.entry(destination_port_id.clone()).or_default().insert(
                    super::PhysicalOverlayDestinationPort::as_concrete(destination_port_mapping.mapping.as_ref())
                        .unwrap()
                        .clone(),
                );
            }
        }

        for (expected_source_id, expected_source_mapping) in expected_source_ports {
            assert_eq!(expected_source_mapping.dialect_type, dialect_type);
            assert!(collected_expected_source_ports
                .insert(
                    expected_source_id.clone(),
                    super::PhysicalOverlaySourcePort::as_concrete(expected_source_mapping.mapping.as_ref())
                        .unwrap()
                        .clone()
                )
                .is_none())
        }

        for (expected_destination_id, expected_destination_mappings) in expected_destination_ports {
            for expected_destination_mapping in expected_destination_mappings {
                assert_eq!(expected_destination_mapping.dialect_type, dialect_type);
                collected_expected_destination_ports
                    .entry(expected_destination_id.clone())
                    .or_default()
                    .insert(
                        super::PhysicalOverlayDestinationPort::as_concrete(expected_destination_mapping.mapping.as_ref())
                            .unwrap()
                            .clone(),
                    );
            }
        }

        assert_eq!(collected_expected_destination_ports, collected_result_destination_ports);
        assert_eq!(collected_expected_source_ports, collected_result_source_ports);
    }

    fn example_port_id(name: &str) -> crate::ir::interaction::PhysicalPortId {
        let input_source_instance_id = edgeless_api::function_instance::InstanceId {
            node_id: uuid::Uuid::new_v4(),
            function_id: uuid::Uuid::new_v4(),
        };
        crate::ir::interaction::PhysicalPortId {
            instance: input_source_instance_id,
            port: edgeless_api::function_instance::PortId(name.to_string()),
        }
    }

    fn dialect_type() -> crate::ir::interaction::dialect::DialectDescriptor {
        crate::ir::interaction::dialect::DialectDescriptor {
            base_type: super::ID,
            constraints: std::collections::BTreeSet::new(),
        }
    }
}
