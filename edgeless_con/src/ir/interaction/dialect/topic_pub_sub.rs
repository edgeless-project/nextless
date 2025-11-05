// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

use crate::ir::interaction::dialect::{AsConcreteDestinationPort, AsConcreteInteraction, AsConcreteSourcePort};

pub static ID: super::DialectId = super::DialectId("TOPIC_PUB_SUB");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopicPubSubSourcePort {
    pub topic: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopicPubSubDestinationPort {
    pub filter: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopicPubSubInteraction {
    sources: Vec<Publisher>,
    subscribers: Vec<Subscriber>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Publisher {
    port: crate::ir::interaction::LogicalPortId,
    topic: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Subscriber {
    port: crate::ir::interaction::LogicalPortId,
    filter: String,
}

// #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
// pub enum TopicPubSubConstraint {}

// impl super::DialectConstraint for TopicPubSubConstraint {
//     fn as_container(self) -> super::DialectConstraintContainer {
//         super::DialectConstraintContainer {
//             dialect: ID,
//             constraint: Box::new(self),
//         }
//     }
// }

pub struct TopicPubSubDialect {}

impl super::DestinationPort for TopicPubSubDestinationPort {}
impl super::SourcePort for TopicPubSubSourcePort {}
impl super::Interaction for TopicPubSubInteraction {
    fn as_physical(&self) -> Option<&dyn super::PhysicalInteraction> {
        None
    }
}

impl super::InteractionPortUtils<crate::ir::interaction::LogicalPortId> for TopicPubSubDialect {
    fn ports_to_interaction(
        &self,
        srcs: Vec<(crate::ir::interaction::LogicalPortId, super::super::SourcePortMapping)>,
        dests: Vec<(crate::ir::interaction::LogicalPortId, super::super::DestiantionPortMapping)>,
    ) -> Result<Vec<super::super::InteractionMapping>, crate::ir::interaction::InteractionError> {
        let mut collector =
            std::collections::HashMap::<String, (Vec<crate::ir::interaction::LogicalPortId>, Vec<crate::ir::interaction::LogicalPortId>)>::new();

        for (logical_port_id, port_spec) in srcs {
            let topic_mapping = TopicPubSubSourcePort::as_concrete(port_spec.mapping.as_ref())?;
            collector
                .entry(topic_mapping.topic.clone())
                .or_insert((Vec::new(), Vec::new()))
                .0
                .push(logical_port_id);
        }

        for (logical_port_id, port_spec) in dests {
            let topic_mapping = TopicPubSubDestinationPort::as_concrete(port_spec.mapping.as_ref())?;
            collector
                .entry(topic_mapping.filter.clone())
                .or_insert((Vec::new(), Vec::new()))
                .1
                .push(logical_port_id);
        }

        Ok(collector
            .into_iter()
            .map(|(topic, (publishers, subscribers))| super::super::InteractionMapping {
                mapping: Box::new(TopicPubSubInteraction {
                    sources: publishers
                        .into_iter()
                        .map(|p_port| Publisher {
                            port: p_port,
                            topic: topic.clone(),
                        })
                        .collect(),
                    subscribers: subscribers
                        .into_iter()
                        .map(|p_port| Subscriber {
                            port: p_port,
                            filter: topic.clone(),
                        })
                        .collect(),
                }),
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
            Vec<(crate::ir::interaction::LogicalPortId, super::super::SourcePortMapping)>,
            Vec<(crate::ir::interaction::LogicalPortId, super::super::DestiantionPortMapping)>,
        ),
        crate::ir::interaction::InteractionError,
    > {
        let topic_mapping = TopicPubSubInteraction::as_concrete(interaction.mapping.as_ref())?;

        let mut sources = Vec::new();
        let mut dests = Vec::new();

        for source in &topic_mapping.sources {
            sources.push((
                source.port.clone(),
                super::super::SourcePortMapping {
                    mapping: Box::new(TopicPubSubSourcePort { topic: source.topic.clone() }),
                    dialect_type: interaction.dialect_type.clone(),
                },
            ));
        }

        for subscriber in &topic_mapping.subscribers {
            dests.push((
                subscriber.port.clone(),
                super::super::DestiantionPortMapping {
                    mapping: Box::new(TopicPubSubDestinationPort {
                        filter: subscriber.filter.clone(),
                    }),
                    dialect_type: interaction.dialect_type.clone(),
                },
            ));
        }

        Ok((sources, dests))
    }
}

impl super::InteractionDialect for TopicPubSubDialect {
    fn id(&self) -> super::DialectId {
        ID
    }

    fn provides_transformations_to(&self) -> &'static [super::DialectId] {
        static TRANSFORMATIONS: [super::DialectId; 1] = [super::logical_overlay::ID];
        &TRANSFORMATIONS
    }

    fn provides_transformations_from(&self) -> &'static [super::DialectId] {
        &[]
    }

    fn plan_translation_to(
        &self,
        src: &super::super::InteractionMapping,
        dst: &super::DialectDescriptor,
    ) -> Result<(super::DialectDescriptor, u64), crate::ir::interaction::InteractionError> {
        assert!(src.dialect_type.base_type == ID);
        if dst.base_type == super::logical_overlay::ID && dst.constraints.is_empty() {
            return Ok((dst.clone(), 1));
        }
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
        if src.dialect_type.base_type != ID || dest.base_type != super::logical_overlay::ID {
            return Err(crate::ir::interaction::InteractionError::UnsupportedTranslation(
                src.dialect_type.clone(),
                dest.clone(),
            ));
        }

        let topic_interaction = TopicPubSubInteraction::as_concrete(src.mapping.as_ref())?;

        let out_mapping = super::logical_overlay::LogicalOverlayInteraction {
            sources: topic_interaction.sources.iter().map(|p| p.port.clone()).collect(),
            destination: super::logical_overlay::DestinationMapping::Multicast(
                topic_interaction.subscribers.iter().map(|s| s.port.clone()).collect(),
            ),
        };

        let out_interaction = super::super::InteractionMapping {
            dialect_type: dest.clone(),
            mapping: Box::new(out_mapping),
        };

        return Ok(vec![out_interaction]);
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
    ) -> crate::ir::interaction::LinkConfigurationResult {
        crate::ir::interaction::LinkConfigurationResult::NoConfig
    }
}
