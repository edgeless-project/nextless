// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

pub mod dialect;

#[derive(thiserror::Error, Debug)]
pub enum BehaviorError {
    #[error("Behavior Dialect \"{0}\" is unknown to the system.")]
    UnknownDialect(String),
    #[error("Feature \"{0}\" is not defined for dialect \"{1}\".")]
    UnknownFeature(String, String),
    #[error("System currently requires a root image. Missing for {0:?}.")]
    NoRootImage(BehaviorId),
    #[error("{0:?} cannot be translated into {1:?}.")]
    UnsupportedTranslation(dialect::DialectId, dialect::DialectId),
    #[error("Cannot map {2:?} while translating {0:?}->{1:?}.")]
    UnsupportedFeatureTranslation(dialect::DialectId, dialect::DialectId, String),
    #[error("Features not supported by target: {0}")]
    UnsupportedFeatures(String),
    #[error("Translation Failed.")]
    TranslationError(#[from] anyhow::Error),
    #[error("Cannot Optimize Ports for {0:?}")]
    UnsupportedOptimization(dialect::DialectId),
}

pub type BehaviorId = edgeless_api::behavior::BehaviorId;
pub type BehaviorSpec = edgeless_api::behavior::BehaviorSpec;
pub type EnabledPorts = edgeless_api::behavior::EnabledPorts;

#[derive(Debug, Clone)]
pub struct Behavior {
    pub(crate) spec: BehaviorSpec,
    pub(crate) main_image: BehaviorImage,
    pub(crate) extra_images: Vec<BehaviorImage>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BehaviorImageId {
    pub(crate) behavior_id: BehaviorId,
    pub(crate) enabled_ports: EnabledPorts,
    pub(crate) dialect_type: dialect::DialectType,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BehaviorImage {
    pub(crate) behavior_image_id: BehaviorImageId,
    pub(crate) image: Vec<u8>,
}

impl TryFrom<edgeless_api::behavior::Behavior> for Behavior {
    type Error = BehaviorError;

    fn try_from(value: edgeless_api::behavior::Behavior) -> Result<Self, Self::Error> {
        Ok(Self {
            spec: value.spec.clone(),
            main_image: value
                .main_image
                .clone()
                .ok_or(BehaviorError::NoRootImage(value.spec.behavior_id.clone()))?
                .try_into()?,
            extra_images: value
                .extra_images
                .into_iter()
                .map(|i| i.try_into())
                .collect::<Result<Vec<_>, Self::Error>>()?,
        })
    }
}

impl TryFrom<edgeless_api::behavior::BehaviorImageId> for BehaviorImageId {
    type Error = BehaviorError;

    fn try_from(value: edgeless_api::behavior::BehaviorImageId) -> Result<Self, Self::Error> {
        Ok(Self {
            behavior_id: value.behaviour_id,
            enabled_ports: value.enabled_ports,
            dialect_type: value.dialect_type.try_into()?,
        })
    }
}

impl TryFrom<edgeless_api::behavior::BehaviorImage> for BehaviorImage {
    type Error = BehaviorError;

    fn try_from(value: edgeless_api::behavior::BehaviorImage) -> Result<Self, Self::Error> {
        Ok(BehaviorImage {
            behavior_image_id: value.behavior_image_id.try_into()?,
            image: value.image,
        })
    }
}

impl TryFrom<edgeless_api::node_registration::RuntimeType> for dialect::DialectType {
    type Error = BehaviorError;

    fn try_from(value: edgeless_api::node_registration::RuntimeType) -> Result<Self, Self::Error> {
        let reg = dialect::DialectRegistry::new_default();

        let dialect = reg
            .get_dialect_for_str(&value.base_type)
            .ok_or(BehaviorError::UnknownDialect(value.base_type.to_string()))?;

        let features = dialect.parse_features(&value.features.iter().map(|f| f.as_str()).collect::<Vec<_>>())?;

        Ok(Self {
            base_type: dialect.id(),
            features,
        })
    }
}
