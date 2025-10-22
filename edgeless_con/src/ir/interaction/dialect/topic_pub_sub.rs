// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

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

pub enum TopicPubSubConstraint {
    None,
}

pub struct TopicPubSubDialect {}

impl super::DestinationPort for TopicPubSubDestinationPort {}
impl super::SourcePort for TopicPubSubSourcePort {}
impl super::Interaction for TopicPubSubInteraction {}

impl super::InteractionDialect<crate::ir::interaction::LogicalPortId, TopicPubSubSourcePort, TopicPubSubDestinationPort, TopicPubSubInteraction>
    for TopicPubSubDialect
{
    fn id(&self) -> super::DialectId {
        ID
    }

    fn provides_transformations_to(&self) -> &'static [super::DialectId] {
        static TRANSFORMATIONS: [super::DialectId; 1] = [super::logical_overlay::ID];
        &TRANSFORMATIONS
    }

    fn plan_translation_to(&self, src: super::DialectDescriptor, dst: super::DialectDescriptor) -> Result<super::DialectDescriptor, ()> {
        todo!()
    }

    fn ports_to_interaction(
        &self,
        srcs: Vec<(crate::ir::interaction::LogicalPortId, TopicPubSubSourcePort)>,
        dests: Vec<(crate::ir::interaction::LogicalPortId, TopicPubSubDestinationPort)>,
    ) -> Vec<TopicPubSubInteraction> {
        let mut collector =
            std::collections::HashMap::<String, (Vec<crate::ir::interaction::LogicalPortId>, Vec<crate::ir::interaction::LogicalPortId>)>::new();

        for (logical_port_id, port_spec) in srcs {
            collector
                .entry(port_spec.topic)
                .or_insert((Vec::new(), Vec::new()))
                .0
                .push(logical_port_id);
        }

        for (logical_port_id, port_spec) in dests {
            collector
                .entry(port_spec.filter)
                .or_insert((Vec::new(), Vec::new()))
                .1
                .push(logical_port_id);
        }

        collector
            .into_iter()
            .map(|(topic, (publishers, subscribers))| TopicPubSubInteraction {
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
            })
            .collect()
    }

    fn interaction_to_ports(
        &self,
        interaction: TopicPubSubInteraction,
    ) -> (
        Vec<(crate::ir::interaction::LogicalPortId, TopicPubSubSourcePort)>,
        Vec<(crate::ir::interaction::LogicalPortId, TopicPubSubDestinationPort)>,
    ) {
        let mut sources = Vec::new();
        let mut dests = Vec::new();

        for source in interaction.sources {
            sources.push((source.port, TopicPubSubSourcePort { topic: source.topic }));
        }

        for subscriber in interaction.subscribers {
            dests.push((subscriber.port, TopicPubSubDestinationPort { filter: subscriber.filter }));
        }

        (sources, dests)
    }
}

impl TopicPubSubDialect {
    pub fn translate_to_logical_overlay(src: TopicPubSubInteraction) -> Vec<super::logical_overlay::LogicalOverlayInteraction> {
        let out = super::logical_overlay::LogicalOverlayInteraction {
            sources: src.sources.into_iter().map(|p| p.port).collect(),
            destination: super::logical_overlay::DestinationMapping::Multicast(src.subscribers.into_iter().map(|s| s.port).collect()),
        };

        vec![out]
    }
}
