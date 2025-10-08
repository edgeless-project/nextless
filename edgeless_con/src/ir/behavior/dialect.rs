// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

pub mod native_dyanamic;
pub mod rust;
pub mod wasm;

pub trait BehaviorDialect {
    fn id(&self) -> DialectId;
    fn provides_transformations_to(&self) -> &'static [DialectId];
    fn provides_transformations_from(&self) -> &'static [DialectId];
    fn plan_transformation_to(&self, source: &DialectType, dest: &DialectType) -> Result<DialectType, super::BehaviorError>;
    fn execute_transformation_to(
        &self,
        base_image: &super::BehaviorImage,
        dest_image_ident: &super::BehaviorImageId,
    ) -> Result<super::BehaviorImage, super::BehaviorError>;
    fn parse_features(&self, feature_strings: &[&str]) -> Result<std::collections::BTreeSet<DialectFeature>, super::BehaviorError>;
    fn check_constraints(&self, desired: &DialectType, constraint: &DialectType) -> Result<(), super::BehaviorError>;
    fn can_self_optimize(&self) -> bool;
}

pub trait ImplicitFeature {
    fn enable_implicitly(&self) -> bool;
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DialectType {
    pub base_type: DialectId,
    pub features: std::collections::BTreeSet<DialectFeature>,
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub struct DialectId(&'static str);

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum DialectFeature {
    Rust(rust::RustDialectFeatures),
    Wasm(wasm::WasmDialectFeatures),
    Native(native_dyanamic::NativeDynamicDialectFeatures),
}

pub struct DialectRegistry {
    registry: std::collections::HashMap<String, Box<dyn BehaviorDialect>>,
}

impl DialectRegistry {
    pub fn new_default() -> Self {
        Self {
            registry: std::collections::HashMap::from([
                (wasm::ID.0.to_string(), Box::new(wasm::WasmDialect {}) as Box<dyn BehaviorDialect>),
                (
                    native_dyanamic::ID.0.to_string(),
                    Box::new(native_dyanamic::NativeDynamicDialect {}) as Box<dyn BehaviorDialect>,
                ),
                (rust::ID.0.to_string(), Box::new(rust::RustDialect {}) as Box<dyn BehaviorDialect>),
            ]),
        }
    }

    pub fn get_dialect_for_str(&self, id_str: &str) -> Option<&dyn BehaviorDialect> {
        self.registry.get(&id_str.to_string()).map(|b| b.as_ref())
    }

    pub fn plan_translation(
        &self,
        source: &super::BehaviorImageId,
        dest: &super::BehaviorImageId,
        require_self_optimization: bool,
    ) -> Result<super::BehaviorImageId, super::BehaviorError> {
        let source_dialect_type = &source.dialect_type;
        let dest_dialect_type = &dest.dialect_type;

        let source_dialect = self
            .registry
            .get(&source_dialect_type.base_type.0.to_string())
            .ok_or(super::BehaviorError::UnknownDialect(source_dialect_type.base_type.0.to_string()))?;
        let dest_dialect = self
            .registry
            .get(&dest_dialect_type.base_type.0.to_string())
            .ok_or(super::BehaviorError::UnknownDialect(dest_dialect_type.base_type.0.to_string()))?;

        if source_dialect_type.base_type == dest_dialect_type.base_type {
            dest_dialect.check_constraints(source_dialect_type, dest_dialect_type)?;

            if require_self_optimization && source.enabled_ports != dest.enabled_ports && !dest_dialect.can_self_optimize() {
                return Err(super::BehaviorError::UnsupportedOptimization(dest_dialect_type.base_type));
            }

            Ok(source.clone())
        } else if source_dialect
            .provides_transformations_to()
            .iter()
            .any(|supported| supported == &dest_dialect_type.base_type)
        {
            let translation = source_dialect.plan_transformation_to(source_dialect_type, dest_dialect_type)?;
            dest_dialect.check_constraints(&translation, dest_dialect_type)?;
            let mut translated_image_id = dest.clone();
            translated_image_id.dialect_type = translation;
            Ok(translated_image_id)
        } else if dest_dialect
            .provides_transformations_from()
            .iter()
            .any(|supported| supported == &source_dialect_type.base_type)
        {
            panic!("plan_transform_from is not implemented yet");
        } else {
            Err(super::BehaviorError::UnsupportedTranslation(
                source_dialect_type.base_type,
                dest_dialect_type.base_type,
            ))
        }
    }

    pub fn try_translate(
        &self,
        source_image: &super::BehaviorImage,
        dest_image_ident: &super::BehaviorImageId,
    ) -> Result<super::BehaviorImage, super::BehaviorError> {
        let source_dialect_type = &source_image.behavior_image_id.dialect_type;
        let dest_dialect_type = &dest_image_ident.dialect_type;

        let source_dialect = self
            .registry
            .get(&source_dialect_type.base_type.0.to_string())
            .ok_or(super::BehaviorError::UnknownDialect(source_dialect_type.base_type.0.to_string()))?;
        let dest_dialect = self
            .registry
            .get(&dest_dialect_type.base_type.0.to_string())
            .ok_or(super::BehaviorError::UnknownDialect(dest_dialect_type.base_type.0.to_string()))?;

        if source_dialect
            .provides_transformations_to()
            .iter()
            .any(|supported| supported == &dest_dialect_type.base_type)
        {
            source_dialect.execute_transformation_to(source_image, dest_image_ident)
        } else if dest_dialect
            .provides_transformations_from()
            .iter()
            .any(|supported| supported == &source_dialect_type.base_type)
        {
            panic!("transform_from is not implemented yet");
        } else {
            Err(super::BehaviorError::UnsupportedTranslation(
                source_dialect_type.base_type,
                dest_dialect_type.base_type,
            ))
        }
    }
}

impl ImplicitFeature for DialectFeature {
    fn enable_implicitly(&self) -> bool {
        match self {
            DialectFeature::Rust(rust_dialect_features) => rust_dialect_features.enable_implicitly(),
            DialectFeature::Wasm(wasm_runtime_features) => wasm_runtime_features.enable_implicitly(),
            DialectFeature::Native(native_runtime_features) => native_runtime_features.enable_implicitly(),
        }
    }
}

impl std::fmt::Display for DialectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

impl std::fmt::Display for DialectFeature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DialectFeature::Rust(rust_dialect_features) => rust_dialect_features.fmt(f),
            DialectFeature::Wasm(wasm_dialect_features) => wasm_dialect_features.fmt(f),
            DialectFeature::Native(native_dynamic_dialect_features) => native_dynamic_dialect_features.fmt(f),
        }
    }
}
