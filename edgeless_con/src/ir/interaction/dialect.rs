// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

use std::fmt::Debug;

pub mod ip_multicast;
pub mod logical_overlay;
pub mod physical_overlay;
pub mod topic_pub_sub;

pub trait InteractionDialect<PortIdType, SourcePortType: SourcePort, DestinationPortType: DestinationPort, InteractionType: Interaction> {
    fn id(&self) -> DialectId;
    fn provides_transformations_to(&self) -> &'static [DialectId];
    fn ports_to_interaction(&self, srcs: Vec<(PortIdType, SourcePortType)>, dests: Vec<(PortIdType, DestinationPortType)>) -> Vec<InteractionType>;
    fn interaction_to_ports(&self, interaction: InteractionType) -> (Vec<(PortIdType, SourcePortType)>, Vec<(PortIdType, DestinationPortType)>);

    fn plan_translation_to(&self, src: DialectDescriptor, dst: DialectDescriptor) -> Result<DialectDescriptor, ()>;
}

pub trait DestinationPort: Debug + Send + DestinationPortHelpers + std::any::Any {}

// https://stackoverflow.com/a/30353928
// https://quinedot.github.io/rust-learning/dyn-trait-eq.html
pub trait DestinationPortHelpers: std::any::Any {
    fn clone_box(&self) -> Box<dyn DestinationPort>;
    fn dyn_eq(&self, other: &dyn DestinationPortHelpers) -> bool;
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

pub trait SourcePort: Debug + Send + SourcePortHelpers + std::any::Any {}

// https://stackoverflow.com/a/30353928
// https://quinedot.github.io/rust-learning/dyn-trait-eq.html
pub trait SourcePortHelpers: std::any::Any {
    fn clone_box(&self) -> Box<dyn SourcePort>;
    fn dyn_eq(&self, other: &dyn SourcePortHelpers) -> bool;
}

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

pub trait Interaction: Debug + Send {}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub struct DialectId(&'static str);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialectDescriptor {
    pub base_type: DialectId,
    pub constraints: std::collections::BTreeSet<DialectConstraint>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialectConstraint {
    IpMulticast(ip_multicast::IpMulticastConstraint),
}
