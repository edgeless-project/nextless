// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

pub static ID: super::DialectId = super::DialectId("NATIVE_DYNAMIC");

pub struct NativeDynamicDialect {}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NativeDynamicDialectFeatures {
    // We add the architecture as a feature so that we don't have to keep track of it separately.
    // We might still need to introduce a suptype for the native runtime but I did not want to commit to that yet.
    Amd64,
    Aarch64,
    // Real Features
    Aes,
}

impl super::BehaviorDialect for NativeDynamicDialect {
    fn id(&self) -> super::DialectId {
        ID
    }

    fn provides_transformations_to(&self) -> &'static [super::DialectId] {
        &[]
    }

    fn provides_transformations_from(&self) -> &'static [super::DialectId] {
        &[]
    }

    fn parse_features(
        &self,
        feature_strings: &[&str],
    ) -> Result<std::collections::BTreeSet<super::DialectFeature>, crate::ir::behavior::BehaviorError> {
        feature_strings
            .iter()
            .map(|f| {
                serde_json::from_str::<NativeDynamicDialectFeatures>(f)
                    .map_err(|_e| crate::ir::behavior::BehaviorError::UnknownFeature(f.to_string(), ID.0.to_string()))
                    .map(super::DialectFeature::Native)
            })
            .collect()
    }

    fn plan_transformation_to(
        &self,
        source: &super::DialectType,
        dest: &super::DialectType,
    ) -> Result<super::DialectType, crate::ir::behavior::BehaviorError> {
        Err(crate::ir::behavior::BehaviorError::UnsupportedTranslation(
            source.base_type,
            dest.base_type,
        ))
    }

    fn check_constraints(&self, desired: &super::DialectType, constraint: &super::DialectType) -> Result<(), crate::ir::behavior::BehaviorError> {
        if constraint.features.is_superset(&desired.features) {
            return Ok(());
        }

        let missing_features = desired
            .features
            .difference(&constraint.features)
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(",");

        Err(crate::ir::behavior::BehaviorError::UnsupportedFeatures(missing_features))
    }

    fn execute_transformation_to(
        &self,
        base_image: &crate::ir::behavior::BehaviorImage,
        dest_image_ident: &crate::ir::behavior::BehaviorImageId,
    ) -> Result<super::super::BehaviorImage, super::super::BehaviorError> {
        Err(crate::ir::behavior::BehaviorError::UnsupportedTranslation(
            base_image.behavior_image_id.dialect_type.base_type,
            dest_image_ident.dialect_type.base_type,
        ))
    }

    fn can_self_optimize(&self) -> bool {
        false
    }
}

impl super::ImplicitFeature for NativeDynamicDialectFeatures {
    fn enable_implicitly(&self) -> bool {
        match self {
            NativeDynamicDialectFeatures::Amd64 => true,
            NativeDynamicDialectFeatures::Aarch64 => true,
            NativeDynamicDialectFeatures::Aes => true,
        }
    }
}

impl std::fmt::Display for NativeDynamicDialectFeatures {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NativeDynamicDialectFeatures::Amd64 => f.write_str("AMD64"),
            NativeDynamicDialectFeatures::Aarch64 => f.write_str("AARCH64"),
            NativeDynamicDialectFeatures::Aes => f.write_str("AES"),
        }
    }
}

impl std::str::FromStr for NativeDynamicDialectFeatures {
    type Err = super::super::BehaviorError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "AMD64" => Ok(Self::Amd64),
            "AARCH64" => Ok(Self::Aarch64),
            "AES" => Ok(Self::Aes),
            _ => Err(super::super::BehaviorError::UnknownFeature(s.to_string(), ID.0.to_string())),
        }
    }
}
