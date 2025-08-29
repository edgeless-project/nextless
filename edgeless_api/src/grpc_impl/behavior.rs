// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

impl From<crate::behavior::BehaviorId> for super::api::BehaviorId {
    fn from(val: crate::behavior::BehaviorId) -> Self {
        super::api::BehaviorId {
            id: val.id,
            version: val.version,
        }
    }
}

impl TryInto<crate::behavior::BehaviorId> for super::api::BehaviorId {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<crate::behavior::BehaviorId, Self::Error> {
        Ok(crate::behavior::BehaviorId {
            id: self.id,
            version: self.version,
        })
    }
}

impl From<crate::behavior::EnabledPorts> for super::api::EnabledPorts {
    fn from(val: crate::behavior::EnabledPorts) -> Self {
        super::api::EnabledPorts {
            enabled_inputs: val.enabled_inputs.into_iter().map(|i| i.into()).collect(),
            enabled_outputs: val.enabled_outputs.into_iter().map(|i| i.into()).collect(),
        }
    }
}

impl TryInto<crate::behavior::EnabledPorts> for super::api::EnabledPorts {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<crate::behavior::EnabledPorts, Self::Error> {
        Ok(crate::behavior::EnabledPorts {
            // https://stackoverflow.com/a/26370894
            enabled_inputs: self
                .enabled_inputs
                .into_iter()
                .map(crate::function_instance::PortId::try_from)
                .collect::<Result<std::collections::BTreeSet<_>, Self::Error>>()?,
            enabled_outputs: self
                .enabled_outputs
                .into_iter()
                .map(crate::function_instance::PortId::try_from)
                .collect::<Result<std::collections::BTreeSet<_>, Self::Error>>()?,
        })
    }
}

impl From<crate::behavior::BehaviorImageId> for super::api::BehaviorImageId {
    fn from(val: crate::behavior::BehaviorImageId) -> Self {
        super::api::BehaviorImageId {
            behavior_id: Some(val.behaviour_id.into()),
            enabled_ports: Some(val.enabled_ports.into()),
            dialect_type: Some(val.dialect_type.into()),
        }
    }
}

impl TryInto<crate::behavior::BehaviorImageId> for super::api::BehaviorImageId {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<crate::behavior::BehaviorImageId, Self::Error> {
        Ok(crate::behavior::BehaviorImageId {
            behaviour_id: self.behavior_id.ok_or(anyhow::anyhow!("Missing Behavior Id"))?.try_into()?,
            enabled_ports: self.enabled_ports.ok_or(anyhow::anyhow!("Missing Enabled Ports"))?.try_into()?,
            dialect_type: self.dialect_type.ok_or(anyhow::anyhow!("Missing DialectType"))?.try_into()?,
        })
    }
}

impl From<crate::behavior::BehaviorImage> for super::api::BehaviorImage {
    fn from(val: crate::behavior::BehaviorImage) -> Self {
        super::api::BehaviorImage {
            behavior_image_id: Some(val.behavior_image_id.into()),
            image: val.image,
        }
    }
}

impl TryFrom<super::api::BehaviorImage> for crate::behavior::BehaviorImage {
    type Error = anyhow::Error;

    fn try_from(value: super::api::BehaviorImage) -> Result<Self, Self::Error> {
        Ok(crate::behavior::BehaviorImage {
            behavior_image_id: value.behavior_image_id.ok_or(anyhow::anyhow!("Missing Behavior Image Id"))?.try_into()?,
            image: value.image,
        })
    }
}

impl From<crate::behavior::BehaviorSpec> for super::api::BehaviorSpec {
    fn from(val: crate::behavior::BehaviorSpec) -> Self {
        super::api::BehaviorSpec {
            behavior_id: Some(val.behavior_id.into()),
            output_ports: val.output_ports.into_iter().map(|(k, v)| (k.0, v.into())).collect(),
            input_ports: val.input_ports.into_iter().map(|(k, v)| (k.0, v.into())).collect(),
            inner_structure: val
                .inner_structure
                .iter()
                .map(|(input, outputs)| super::api::Mapping {
                    source: Some(match input {
                        crate::function_instance::MappingNode::Port(port_id) => super::api::MappingNodeVariant {
                            mapping_node_type: Some(super::api::mapping_node_variant::MappingNodeType::Port(port_id.0.clone())),
                        },
                        crate::function_instance::MappingNode::SideEffect => super::api::MappingNodeVariant {
                            mapping_node_type: Some(super::api::mapping_node_variant::MappingNodeType::SideEffect(
                                super::api::SideEffectMapping {},
                            )),
                        },
                    }),
                    dest: outputs
                        .iter()
                        .map(|item| match item {
                            crate::function_instance::MappingNode::Port(port_id) => super::api::MappingNodeVariant {
                                mapping_node_type: Some(super::api::mapping_node_variant::MappingNodeType::Port(port_id.0.clone())),
                            },
                            crate::function_instance::MappingNode::SideEffect => super::api::MappingNodeVariant {
                                mapping_node_type: Some(super::api::mapping_node_variant::MappingNodeType::SideEffect(
                                    super::api::SideEffectMapping {},
                                )),
                            },
                        })
                        .collect(),
                })
                .collect(),
        }
    }
}

impl TryFrom<super::api::BehaviorSpec> for crate::behavior::BehaviorSpec {
    type Error = anyhow::Error;

    fn try_from(value: super::api::BehaviorSpec) -> Result<Self, Self::Error> {
        Ok(Self {
            behavior_id: value.behavior_id.ok_or(anyhow::anyhow!("Missing Behavior Id"))?.try_into()?,
            input_ports: value
                .input_ports
                .into_iter()
                .map(|(k, v)| Ok((crate::function_instance::PortId(k), v.try_into()?)))
                .collect::<Result<std::collections::BTreeMap<_, _>, Self::Error>>()?,
            output_ports: value
                .output_ports
                .into_iter()
                .map(|(k, v)| Ok((crate::function_instance::PortId(k), v.try_into()?)))
                .collect::<Result<std::collections::BTreeMap<_, _>, Self::Error>>()?,
            inner_structure: value
                .inner_structure
                .into_iter()
                .map(|mapping| {
                    (
                        match &mapping.source.as_ref().unwrap().mapping_node_type.as_ref().unwrap() {
                            super::api::mapping_node_variant::MappingNodeType::Port(port_id) => {
                                crate::function_instance::MappingNode::Port(crate::function_instance::PortId(port_id.clone()))
                            }
                            super::api::mapping_node_variant::MappingNodeType::SideEffect(_) => crate::function_instance::MappingNode::SideEffect,
                        },
                        mapping
                            .dest
                            .iter()
                            .map(|id| match &id.mapping_node_type.as_ref().unwrap() {
                                super::api::mapping_node_variant::MappingNodeType::Port(port_id) => {
                                    crate::function_instance::MappingNode::Port(crate::function_instance::PortId(port_id.clone()))
                                }
                                super::api::mapping_node_variant::MappingNodeType::SideEffect(_) => crate::function_instance::MappingNode::SideEffect,
                            })
                            .collect(),
                    )
                })
                .collect(),
        })
    }
}

impl From<crate::behavior::Behavior> for super::api::Behavior {
    fn from(val: crate::behavior::Behavior) -> Self {
        super::api::Behavior {
            spec: Some(val.spec.into()),
            main_image: val.main_image.map(|i| i.into()),
            extra_images: val.extra_images.into_iter().map(|i| i.into()).collect(),
        }
    }
}

impl TryFrom<super::api::Behavior> for crate::behavior::Behavior {
    type Error = anyhow::Error;

    fn try_from(value: super::api::Behavior) -> Result<Self, Self::Error> {
        Ok(Self {
            spec: value.spec.ok_or(anyhow::anyhow!("Missing Behavior Spec"))?.try_into()?,
            main_image: match value.main_image {
                Some(image) => Some(image.try_into()?),
                None => None,
            },
            extra_images: value
                .extra_images
                .into_iter()
                .map(|i| i.try_into())
                .collect::<Result<Vec<_>, Self::Error>>()?,
        })
    }
}
