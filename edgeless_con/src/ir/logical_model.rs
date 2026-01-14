// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

pub trait LogicalComponent {
    fn logical_ports(&self) -> &LogicalPorts;
    fn logical_ports_mut(&mut self) -> &mut LogicalPorts;
    fn instance_ids(&mut self) -> Vec<edgeless_api::function_instance::InstanceId>;
    fn instances(&self) -> Vec<&std::cell::RefCell<super::physical_model::PhysicalComponentState>>;
    fn split_view(&mut self) -> (&mut LogicalPorts, Vec<&std::cell::RefCell<super::physical_model::PhysicalComponentState>>);
}

#[derive(Default, Debug, Clone)]
pub struct LogicalPorts {
    pub logical_output_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, crate::ir::interaction::SourcePortMapping>,
    pub logical_input_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, crate::ir::interaction::DestiantionPortMapping>,
}

pub fn parse_api_output_mapping(
    mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, edgeless_api::workflow_instance::PortMapping>,
) -> std::collections::HashMap<edgeless_api::function_instance::PortId, crate::ir::interaction::SourcePortMapping> {
    mapping.into_iter().map(|(port_id, port)| (port_id, port.into())).collect()
}

impl From<edgeless_api::workflow_instance::PortMapping> for crate::ir::interaction::SourcePortMapping {
    fn from(value: edgeless_api::workflow_instance::PortMapping) -> Self {
        match value {
            edgeless_api::workflow_instance::PortMapping::DirectTarget(node, port_id) => crate::ir::interaction::SourcePortMapping {
                dialect_type: crate::ir::interaction::dialect::DialectDescriptor {
                    base_type: crate::ir::interaction::dialect::logical_overlay::ID,
                    constraints: std::collections::BTreeSet::new(),
                },
                mapping: Box::new(crate::ir::interaction::dialect::logical_overlay::LogicalOverlaySourcePort {
                    destination: crate::ir::interaction::dialect::logical_overlay::DestinationMapping::Unicast(
                        crate::ir::interaction::LogicalPortId {
                            component: node,
                            port: port_id,
                        },
                    ),
                }),
            },
            edgeless_api::workflow_instance::PortMapping::AnyOfTargets(items) => crate::ir::interaction::SourcePortMapping {
                dialect_type: crate::ir::interaction::dialect::DialectDescriptor {
                    base_type: crate::ir::interaction::dialect::logical_overlay::ID,
                    constraints: std::collections::BTreeSet::new(),
                },
                mapping: Box::new(crate::ir::interaction::dialect::logical_overlay::LogicalOverlaySourcePort {
                    destination: crate::ir::interaction::dialect::logical_overlay::DestinationMapping::Anycast(
                        items
                            .into_iter()
                            .map(|i| crate::ir::interaction::LogicalPortId { component: i.0, port: i.1 })
                            .collect(),
                    ),
                }),
            },
            edgeless_api::workflow_instance::PortMapping::AllOfTargets(items) => crate::ir::interaction::SourcePortMapping {
                dialect_type: crate::ir::interaction::dialect::DialectDescriptor {
                    base_type: crate::ir::interaction::dialect::logical_overlay::ID,
                    constraints: std::collections::BTreeSet::new(),
                },
                mapping: Box::new(crate::ir::interaction::dialect::logical_overlay::LogicalOverlaySourcePort {
                    destination: crate::ir::interaction::dialect::logical_overlay::DestinationMapping::Multicast(
                        items
                            .into_iter()
                            .map(|i| crate::ir::interaction::LogicalPortId { component: i.0, port: i.1 })
                            .collect(),
                    ),
                }),
            },
            edgeless_api::workflow_instance::PortMapping::Topic(topic) => crate::ir::interaction::SourcePortMapping {
                dialect_type: crate::ir::interaction::dialect::DialectDescriptor {
                    base_type: crate::ir::interaction::dialect::topic_pub_sub::ID,
                    constraints: std::collections::BTreeSet::new(),
                },
                mapping: Box::new(crate::ir::interaction::dialect::topic_pub_sub::TopicPubSubSourcePort { topic: topic }),
            },
        }
    }
}

pub fn parse_api_input_mapping(
    mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, edgeless_api::workflow_instance::PortMapping>,
) -> std::collections::HashMap<edgeless_api::function_instance::PortId, crate::ir::interaction::DestiantionPortMapping> {
    mapping
        .into_iter()
        .filter_map(|(port_id, port)| match port.try_into() {
            Ok(port) => Some((port_id, port)),
            Err(_) => None,
        })
        .collect()
}

impl TryFrom<edgeless_api::workflow_instance::PortMapping> for crate::ir::interaction::DestiantionPortMapping {
    type Error = anyhow::Error;

    fn try_from(value: edgeless_api::workflow_instance::PortMapping) -> Result<Self, Self::Error> {
        match value {
            edgeless_api::workflow_instance::PortMapping::Topic(topic) => Ok(crate::ir::interaction::DestiantionPortMapping {
                dialect_type: crate::ir::interaction::dialect::DialectDescriptor {
                    base_type: crate::ir::interaction::dialect::topic_pub_sub::ID,
                    constraints: std::collections::BTreeSet::new(),
                },
                mapping: Box::new(crate::ir::interaction::dialect::topic_pub_sub::TopicPubSubDestinationPort { filter: topic }),
            }),
            _ => Err(anyhow::anyhow!("Bad Input Port Mapping")),
        }
    }
}

pub type LogicalInput = crate::ir::interaction::DestiantionPortMapping;
pub type LogicalOutput = crate::ir::interaction::SourcePortMapping;
