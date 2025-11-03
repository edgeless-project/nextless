// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

use std::fmt::Debug;

pub mod ip_multicast;
pub mod logical_overlay;
pub mod physical_overlay;
pub mod topic_pub_sub;

pub trait InteractionDialect: Sync + Send {
    fn id(&self) -> DialectId;
    fn provides_transformations_to(&self) -> &'static [DialectId];
    fn provides_transformations_from(&self) -> &'static [DialectId];

    fn plan_translation_to(&self, src: &DialectDescriptor, dst: &DialectDescriptor) -> Result<DialectDescriptor, ()>;
    fn execute_transformation_to(&self, src: &super::InteractionMapping, dest: &DialectDescriptor) -> Result<Vec<super::InteractionMapping>, ()>;

    fn logical_utils(&self) -> Option<&dyn InteractionPortUtils<super::LogicalPortId>>;
    fn physical_utils(&self) -> Option<&dyn InteractionPortUtils<super::PhysicalPortId>>;

    fn link_config(
        &self,
        mapping: &super::InteractionMapping,
        nodes: &std::collections::HashMap<uuid::Uuid, &dyn crate::ir::Node>,
    ) -> crate::ir::link::WorkflowLink;
}

pub trait InteractionPortUtils<PortIdType> {
    fn ports_to_interaction(
        &self,
        srcs: Vec<(PortIdType, super::SourcePortMapping)>,
        dests: Vec<(PortIdType, super::DestiantionPortMapping)>,
    ) -> Vec<super::InteractionMapping>;
    fn interaction_to_ports(
        &self,
        interaction: super::InteractionMapping,
    ) -> (
        Vec<(PortIdType, super::SourcePortMapping)>,
        Vec<(PortIdType, super::DestiantionPortMapping)>,
    );
}

pub struct DialectRegistry {
    registry: std::collections::HashMap<String, Box<dyn InteractionDialect>>,
}

impl DialectRegistry {
    pub fn new_default() -> Self {
        Self {
            registry: std::collections::HashMap::from([
                (
                    ip_multicast::ID.0.to_string(),
                    Box::new(ip_multicast::IpMulticastDialect::new()) as Box<dyn InteractionDialect>,
                ),
                (
                    logical_overlay::ID.0.to_string(),
                    Box::new(logical_overlay::LogicalOverlayDialect {}) as Box<dyn InteractionDialect>,
                ),
                (
                    physical_overlay::ID.0.to_string(),
                    Box::new(physical_overlay::PhysicalOverlayDialect {}) as Box<dyn InteractionDialect>,
                ),
                (
                    topic_pub_sub::ID.0.to_string(),
                    Box::new(topic_pub_sub::TopicPubSubDialect {}) as Box<dyn InteractionDialect>,
                ),
            ]),
        }
    }

    pub fn logical_ports_to_interaction(
        &self,
        dialect_id: &DialectDescriptor,
        srcs: Vec<(super::LogicalPortId, super::SourcePortMapping)>,
        dests: Vec<(super::LogicalPortId, super::DestiantionPortMapping)>,
    ) -> Result<Vec<super::InteractionMapping>, ()> {
        let dialect: &dyn InteractionDialect = self.registry.get(&dialect_id.base_type.0.to_string()).ok_or(())?.as_ref();

        let port_converter = dialect.logical_utils().ok_or(())?;

        Ok(port_converter.ports_to_interaction(srcs, dests))
    }

    pub fn logical_interaction_to_ports(
        &self,
        interaction: super::InteractionMapping,
    ) -> Result<
        (
            Vec<(super::LogicalPortId, super::SourcePortMapping)>,
            Vec<(super::LogicalPortId, super::DestiantionPortMapping)>,
        ),
        (),
    > {
        let dialect: &dyn InteractionDialect = self.registry.get(&interaction.dialect_type.base_type.0.to_string()).ok_or(())?.as_ref();

        let port_converter = dialect.logical_utils().ok_or(())?;

        Ok(port_converter.interaction_to_ports(interaction))
    }

    pub fn physical_ports_to_interaction(
        &self,
        dialect_id: &DialectDescriptor,
        srcs: Vec<(super::PhysicalPortId, super::SourcePortMapping)>,
        dests: Vec<(super::PhysicalPortId, super::DestiantionPortMapping)>,
    ) -> Result<Vec<super::InteractionMapping>, ()> {
        let dialect: &dyn InteractionDialect = self.registry.get(&dialect_id.base_type.0.to_string()).ok_or(())?.as_ref();

        let port_converter = dialect.physical_utils().ok_or(())?;

        Ok(port_converter.ports_to_interaction(srcs, dests))
    }

    pub fn physical_interaction_to_ports(
        &self,
        interaction: super::InteractionMapping,
    ) -> Result<
        (
            Vec<(super::PhysicalPortId, super::SourcePortMapping)>,
            Vec<(super::PhysicalPortId, super::DestiantionPortMapping)>,
        ),
        (),
    > {
        let dialect: &dyn InteractionDialect = self.registry.get(&interaction.dialect_type.base_type.0.to_string()).ok_or(())?.as_ref();

        let port_converter = dialect.physical_utils().ok_or(())?;

        Ok(port_converter.interaction_to_ports(interaction))
    }

    pub fn plan_translation(&mut self, source: &DialectDescriptor, dest: &DialectDescriptor) -> Result<DialectDescriptor, ()> {
        let source_dialect: &dyn InteractionDialect = self.registry.get(&source.base_type.0.to_string()).ok_or(())?.as_ref();
        let destination_dialect: &dyn InteractionDialect = self.registry.get(&dest.base_type.0.to_string()).ok_or(())?.as_ref();

        if source_dialect.provides_transformations_to().contains(&dest.base_type) {
            return source_dialect.plan_translation_to(source, dest);
        } else if destination_dialect.provides_transformations_from().contains(&source.base_type) {
            return destination_dialect.plan_translation_to(source, dest);
        }

        Err(())
    }

    pub fn try_translate(&mut self, source: &super::InteractionMapping, dest: &DialectDescriptor) -> Result<Vec<super::InteractionMapping>, ()> {
        let source_dialect: &dyn InteractionDialect = self.registry.get(&source.dialect_type.base_type.0.to_string()).ok_or(())?.as_ref();
        let destination_dialect: &dyn InteractionDialect = self.registry.get(&dest.base_type.0.to_string()).ok_or(())?.as_ref();

        if source_dialect.provides_transformations_to().contains(&dest.base_type) {
            return source_dialect.execute_transformation_to(source, dest);
        } else if destination_dialect
            .provides_transformations_from()
            .contains(&source.dialect_type.base_type)
        {
            return destination_dialect.execute_transformation_to(source, dest);
        }

        Err(())
    }

    pub async fn instantiate_link_data_plane(
        &mut self,
        _link_id: edgeless_api::link::LinkInstanceId,
        _class: edgeless_api::link::LinkType,
    ) -> Result<(), String> {
        // TODO: This exists for compatibility with the older Link Model and will be used to instantitate the global state of a link (i.e. SDN),
        Ok(())
    }
}

pub trait DestinationPort: Debug + Send + DestinationPortHelpers + std::any::Any {}
pub trait SourcePort: Debug + Send + SourcePortHelpers + std::any::Any {}
pub trait Interaction: Debug + Send + InteractionHelpers + std::any::Any {
    fn as_physical(&self) -> Option<&dyn PhysicalInteraction>;
}

pub trait PhysicalInteraction {
    fn relevant_nodes(&self) -> Vec<edgeless_api::function_instance::NodeId>;
}

trait AsConcreteInteraction {
    fn as_concrete(a: &dyn Interaction) -> Result<&Self, ()>;
}

impl<T: Interaction> AsConcreteInteraction for T {
    fn as_concrete(a: &dyn Interaction) -> Result<&Self, ()> {
        let as_any = a as &dyn std::any::Any;
        as_any.downcast_ref().ok_or(())
    }
}

// https://stackoverflow.com/a/30353928
// https://quinedot.github.io/rust-learning/dyn-trait-eq.html
pub trait DestinationPortHelpers: std::any::Any {
    fn clone_box(&self) -> Box<dyn DestinationPort>;
    fn dyn_eq(&self, other: &dyn DestinationPortHelpers) -> bool;
}

// https://stackoverflow.com/a/30353928
// https://quinedot.github.io/rust-learning/dyn-trait-eq.html
pub trait SourcePortHelpers: std::any::Any {
    fn clone_box(&self) -> Box<dyn SourcePort>;
    fn dyn_eq(&self, other: &dyn SourcePortHelpers) -> bool;
}

pub trait InteractionHelpers: std::any::Any {
    fn clone_box(&self) -> Box<dyn Interaction>;
    fn dyn_eq(&self, other: &dyn InteractionHelpers) -> bool;
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub struct DialectId(&'static str);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DialectDescriptor {
    pub base_type: DialectId,
    pub constraints: std::collections::BTreeSet<DialectConstraint>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum DialectConstraint {
    IpMulticast(ip_multicast::IpMulticastConstraint),
    LogicalOverlay(logical_overlay::LogicalOverlayConstraint),
    PhysicalOverlay(physical_overlay::PhysicalOverlayConstraint),
    TopicPubSub(topic_pub_sub::TopicPubSubConstraint),
}

impl<T> DestinationPortHelpers for T
where
    T: 'static + DestinationPort + Clone + PartialEq + Eq,
{
    fn clone_box(&self) -> Box<dyn DestinationPort> {
        Box::new(self.clone())
    }

    // https://quinedot.github.io/rust-learning/dyn-trait-eq.html
    fn dyn_eq(&self, other: &dyn DestinationPortHelpers) -> bool {
        let other_any = other as &dyn std::any::Any;
        let other_slf = other_any.downcast_ref::<Self>();
        if let Some(other) = other_slf {
            self.eq(other)
        } else {
            false
        }
    }
}
impl Clone for Box<dyn DestinationPort> {
    fn clone(&self) -> Box<dyn DestinationPort> {
        self.clone_box()
    }
}

impl PartialEq for Box<dyn DestinationPort> {
    fn eq(&self, other: &Self) -> bool {
        self.dyn_eq(other.as_ref())
    }
}

impl Eq for Box<dyn DestinationPort> {}

impl<T> SourcePortHelpers for T
where
    T: 'static + SourcePort + Clone + PartialEq + Eq,
{
    fn clone_box(&self) -> Box<dyn SourcePort> {
        Box::new(self.clone())
    }

    // https://quinedot.github.io/rust-learning/dyn-trait-eq.html
    fn dyn_eq(&self, other: &dyn SourcePortHelpers) -> bool {
        let other_any = other as &dyn std::any::Any;
        let other_slf = other_any.downcast_ref::<Self>();
        if let Some(other) = other_slf {
            self.eq(other)
        } else {
            false
        }
    }
}
impl Clone for Box<dyn SourcePort> {
    fn clone(&self) -> Box<dyn SourcePort> {
        self.clone_box()
    }
}

impl PartialEq for Box<dyn SourcePort> {
    fn eq(&self, other: &Self) -> bool {
        self.dyn_eq(other.as_ref())
    }
}

impl Eq for Box<dyn SourcePort> {}

impl<T> InteractionHelpers for T
where
    T: 'static + Interaction + Clone + PartialEq + Eq,
{
    fn clone_box(&self) -> Box<dyn Interaction> {
        Box::new(self.clone())
    }

    // https://quinedot.github.io/rust-learning/dyn-trait-eq.html
    fn dyn_eq(&self, other: &dyn InteractionHelpers) -> bool {
        let other_any = other as &dyn std::any::Any;
        let other_slf = other_any.downcast_ref::<Self>();
        if let Some(other) = other_slf {
            self.eq(other)
        } else {
            false
        }
    }
}
impl Clone for Box<dyn Interaction> {
    fn clone(&self) -> Box<dyn Interaction> {
        self.clone_box()
    }
}

impl PartialEq for Box<dyn Interaction> {
    fn eq(&self, other: &Self) -> bool {
        self.dyn_eq(other.as_ref())
    }
}

impl Eq for Box<dyn Interaction> {}
