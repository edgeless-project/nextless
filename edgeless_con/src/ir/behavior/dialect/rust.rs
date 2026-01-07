// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-FileCopyrightText: © 2023 University of Cambridge, System Research Group
// SPDX-FileCopyrightText: © 2024 Roman Kolcun <roman.kolcun@cl.cam.ac.uk>
// SPDX-License-Identifier: MIT

use std::str::FromStr;

use crate::ir::behavior::dialect::ImplicitFeature;

pub static ID: super::DialectId = super::DialectId("RUST");

pub struct RustDialect {}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RustDialectFeatures {
    Wgpu,
    NoStd,
}

impl super::BehaviorDialect for RustDialect {
    fn id(&self) -> super::DialectId {
        ID
    }

    fn provides_transformations_to(&self) -> &'static [super::DialectId] {
        static TRANSFORMATIONS: [super::DialectId; 2] = [super::wasm::ID, super::native_dyanamic::ID];
        &TRANSFORMATIONS
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
                RustDialectFeatures::from_str(f)
                    .map_err(|_e| crate::ir::behavior::BehaviorError::UnknownFeature(f.to_string(), ID.0.to_string()))
                    .map(super::DialectFeature::Rust)
            })
            .collect()
    }

    fn plan_transformation_to(
        &self,
        source: &super::DialectType,
        dest: &super::DialectType,
    ) -> Result<super::DialectType, crate::ir::behavior::BehaviorError> {
        assert!(source.base_type == ID);

        if dest.base_type == super::wasm::ID {
            return self.plan_transformation_to_wasm(source, dest);
        }

        if dest.base_type == super::native_dyanamic::ID {
            return self.plan_transformation_to_native_dynamic(source, dest);
        }

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
        assert!(base_image.behavior_image_id.dialect_type.base_type == ID);

        if dest_image_ident.dialect_type.base_type == super::wasm::ID {
            return self.transform_to_wasm(base_image, dest_image_ident);
        }

        if dest_image_ident.dialect_type.base_type == super::native_dyanamic::ID {
            return self.transform_to_native(base_image, dest_image_ident);
        }

        Err(crate::ir::behavior::BehaviorError::UnsupportedTranslation(
            base_image.behavior_image_id.dialect_type.base_type,
            dest_image_ident.dialect_type.base_type,
        ))
    }

    fn can_self_optimize(&self) -> bool {
        false
    }
}

impl RustDialect {
    fn plan_transformation_to_wasm(
        &self,
        source: &super::DialectType,
        dest: &super::DialectType,
    ) -> Result<super::DialectType, crate::ir::behavior::BehaviorError> {
        assert!(source.base_type == ID);
        assert!(dest.base_type == super::wasm::ID);

        let translated_features: std::collections::BTreeSet<crate::ir::behavior::dialect::DialectFeature> = source
            .features
            .iter()
            .filter_map(|f| match f {
                crate::ir::behavior::dialect::DialectFeature::Rust(rust_dialect_feature) => match rust_dialect_feature {
                    crate::ir::behavior::dialect::rust::RustDialectFeatures::Wgpu => {
                        if dest.features.contains(&crate::ir::behavior::dialect::DialectFeature::Wasm(
                            crate::ir::behavior::dialect::wasm::WasmDialectFeatures::Wgpu,
                        )) {
                            Some(Ok(crate::ir::behavior::dialect::DialectFeature::Wasm(
                                crate::ir::behavior::dialect::wasm::WasmDialectFeatures::Wgpu,
                            )))
                        } else {
                            Some(Err(crate::ir::behavior::BehaviorError::UnsupportedFeatureTranslation(
                                source.base_type,
                                dest.base_type,
                                "WGPU".to_string(),
                            )))
                        }
                    }
                    RustDialectFeatures::NoStd => None,
                },
                _ => Some(Err(crate::ir::behavior::BehaviorError::UnsupportedFeatureTranslation(
                    source.base_type,
                    dest.base_type,
                    String::new(),
                ))),
            })
            .collect::<Result<std::collections::BTreeSet<crate::ir::behavior::dialect::DialectFeature>, crate::ir::behavior::BehaviorError>>()?;

        let mut enabled_features = translated_features;
        for feature in &dest.features {
            if feature.enable_implicitly() {
                enabled_features.insert(feature.clone());
            }
        }

        Ok(super::DialectType {
            base_type: super::wasm::ID,
            features: enabled_features,
        })
    }

    fn plan_transformation_to_native_dynamic(
        &self,
        source: &super::DialectType,
        dest: &super::DialectType,
    ) -> Result<super::DialectType, crate::ir::behavior::BehaviorError> {
        assert!(source.base_type == ID);
        assert!(dest.base_type == super::native_dyanamic::ID);

        if !source
            .features
            .contains(&crate::ir::behavior::dialect::DialectFeature::Rust(RustDialectFeatures::NoStd))
        {
            return Err(crate::ir::behavior::BehaviorError::UnsupportedTranslation(
                source.base_type,
                dest.base_type,
            ));
        }

        let translated_features: std::collections::BTreeSet<crate::ir::behavior::dialect::DialectFeature> = source
            .features
            .iter()
            .filter_map(|f| match f {
                crate::ir::behavior::dialect::DialectFeature::Rust(rust_dialect_feature) => match rust_dialect_feature {
                    RustDialectFeatures::Wgpu => Some(Err(crate::ir::behavior::BehaviorError::UnsupportedFeatureTranslation(
                        source.base_type,
                        dest.base_type,
                        "WGPU".to_string(),
                    ))),
                    RustDialectFeatures::NoStd => None,
                },
                _ => Some(Err(crate::ir::behavior::BehaviorError::UnsupportedFeatureTranslation(
                    source.base_type,
                    dest.base_type,
                    String::new(),
                ))),
            })
            .collect::<Result<std::collections::BTreeSet<crate::ir::behavior::dialect::DialectFeature>, crate::ir::behavior::BehaviorError>>()?;

        let mut enabled_features = translated_features;
        for feature in &dest.features {
            if feature.enable_implicitly() {
                enabled_features.insert(feature.clone());
            }
        }

        Ok(super::DialectType {
            base_type: super::native_dyanamic::ID,
            features: enabled_features,
        })
    }

    fn transform_to_wasm(
        &self,
        base_image: &super::super::BehaviorImage,
        dest_image_ident: &crate::ir::behavior::BehaviorImageId,
    ) -> Result<crate::ir::behavior::BehaviorImage, crate::ir::behavior::BehaviorError> {
        let enabled_features = port_features_for(dest_image_ident);

        let rust_dir = edgeless_build::rust::unpack_rust_package(&base_image.image)
            .map_err(|e| crate::ir::behavior::BehaviorError::TranslationError(e.into()))?;
        let wasm_file = edgeless_build::wasm::rust_to_wasm(rust_dir, enabled_features, true, false)
            .map_err(|e| crate::ir::behavior::BehaviorError::TranslationError(e.into()))?;
        let wasm_code = std::fs::read(wasm_file)
            .map_err(|e| crate::ir::behavior::BehaviorError::TranslationError(anyhow::anyhow!("Cold not read wasm file: {}", e)))?;

        Ok(crate::ir::behavior::BehaviorImage {
            behavior_image_id: dest_image_ident.clone(),
            image: wasm_code,
        })
    }

    fn transform_to_native(
        &self,
        base_image: &super::super::BehaviorImage,
        dest_image_ident: &crate::ir::behavior::BehaviorImageId,
    ) -> Result<crate::ir::behavior::BehaviorImage, crate::ir::behavior::BehaviorError> {
        let enabled_features = port_features_for(dest_image_ident);

        let mut target = None;

        let features = dest_image_ident
            .dialect_type
            .features
            .iter()
            .filter_map(|feature| match feature {
                crate::ir::behavior::dialect::DialectFeature::Native(native_runtime_feature) => match native_runtime_feature {
                    crate::ir::behavior::dialect::native_dyanamic::NativeDynamicDialectFeatures::Amd64 => {
                        if target.is_some() {
                            tracing::error!("Multiple target architectures!")
                        }
                        target = Some(edgeless_build::native::NativeTarget::AMD64);
                        None
                    }
                    crate::ir::behavior::dialect::native_dyanamic::NativeDynamicDialectFeatures::Aarch64 => {
                        if target.is_some() {
                            tracing::error!("Multiple target architectures!")
                        }
                        target = Some(edgeless_build::native::NativeTarget::AARCH64);
                        None
                    }
                    crate::ir::behavior::dialect::native_dyanamic::NativeDynamicDialectFeatures::Aes => {
                        Some(edgeless_build::native::NativeFeature::Aes)
                    }
                },
                _ => {
                    tracing::error!("Called native build with invalid features.");
                    None
                }
            })
            .collect();

        let target = if let Some(target) = target {
            target
        } else {
            return Err(crate::ir::behavior::BehaviorError::TranslationError(anyhow::anyhow!(
                "Native Build: Unsupported Target"
            )));
        };

        let rust_dir = edgeless_build::rust::unpack_rust_package(&base_image.image)
            .map_err(|e| crate::ir::behavior::BehaviorError::TranslationError(e.into()))?;
        let so_file = edgeless_build::native::rust_to_dynlib(rust_dir, enabled_features, true, false, target, features)
            .map_err(|e| crate::ir::behavior::BehaviorError::TranslationError(e.into()))?;
        let so_code = std::fs::read(so_file)
            .map_err(|e| crate::ir::behavior::BehaviorError::TranslationError(anyhow::anyhow!("Cold not read wasm file: {}", e)))?;

        Ok(crate::ir::behavior::BehaviorImage {
            behavior_image_id: dest_image_ident.clone(),
            image: so_code,
        })
    }
}

impl super::ImplicitFeature for RustDialectFeatures {
    fn enable_implicitly(&self) -> bool {
        match self {
            RustDialectFeatures::Wgpu => false,
            RustDialectFeatures::NoStd => false,
        }
    }
}

fn port_features_for(image_ident: &crate::ir::behavior::BehaviorImageId) -> Vec<String> {
    let mut enabled_features: Vec<String> = Vec::new();
    for input in &image_ident.enabled_ports.enabled_inputs {
        enabled_features.push(format!("input_{}", input.0))
    }
    for output in &image_ident.enabled_ports.enabled_outputs {
        enabled_features.push(format!("output_{}", output.0))
    }

    enabled_features
}

impl std::fmt::Display for RustDialectFeatures {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RustDialectFeatures::Wgpu => f.write_str("WGPU"),
            RustDialectFeatures::NoStd => f.write_str("NO_STD"),
        }
    }
}

impl std::str::FromStr for RustDialectFeatures {
    type Err = super::super::BehaviorError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "WGPU" => Ok(Self::Wgpu),
            "NO_STD" => Ok(Self::NoStd),
            _ => Err(super::super::BehaviorError::UnknownFeature(s.to_string(), ID.0.to_string())),
        }
    }
}
