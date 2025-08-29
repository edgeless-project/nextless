// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

use std::str::FromStr;

use crate::ir::ImplicitFeature;

pub fn feasible_node_runtime_candidates<'b>(
    actor: &crate::ir::actor::LogicalActor,
    node: &'b dyn crate::ir::Node,
    new_instance: bool,
) -> Vec<super::Candidate<'b>> {
    let mut candidates = Vec::new();

    if !node_fulfills_constraints(actor, node) {
        return Vec::new();
    }

    if let Some(dest_node) = actor.annotations.get("node_id_init_on") {
        if actor.instances.len() == 1 && new_instance {
            let dest_uuid = uuid::Uuid::from_str(dest_node).unwrap();
            if dest_uuid != node.node_id() {
                return Vec::new();
            }
        }
    }

    for (_rt_id, rt) in node.available_runtimes() {
        if let Some(features) = runtime_supported(
            &actor.image.main_image.behavior_image_id.dialect_type,
            &rt,
            actor.annotations.contains_key("NO_NATIVE"),
        ) {
            candidates.push(super::Candidate {
                node_id: node.node_id(),
                runtime: rt.clone(),
                runtime_features: features,
            });
        }
    }

    candidates
}

fn node_fulfills_constraints(actor: &crate::ir::actor::LogicalActor, node: &dyn crate::ir::Node) -> bool {
    if let Some(viable_domains) = &actor.constraints.domain_id_match_any {
        if !viable_domains.contains(&node.cluster_id()) {
            return false;
        }
    }

    if let Some(viable_nodes) = &actor.constraints.node_id_match_any {
        if !viable_nodes.contains(&node.node_id()) {
            return false;
        }
    }

    for label in actor.constraints.label_match_all.iter() {
        if !node.labels().contains(label) {
            return false;
        }
    }

    for required_resource_class in &actor.constraints.resource_match_all {
        if !node
            .available_resource_providers()
            .iter()
            .any(|r| r.1.class_type().as_str() == required_resource_class.as_str())
        {
            return false;
        }
    }

    true
}

fn runtime_supported(
    source_format: &crate::ir::actor::DialectType,
    runtime: &crate::ir::Runtime,
    disable_native: bool,
) -> Option<std::collections::BTreeSet<crate::ir::DialectFeature>> {
    match runtime {
        super::Runtime::WasmBase(_wasm_runtime, _features) => {
            if !["RUST", "WASM"].contains(&source_format.base_type.as_str()) {
                return None;
            }

            let mut requires_unsupported_feature = false;
            let translated_features: std::collections::BTreeSet<_> = source_format
                .features
                .iter()
                .filter_map(|f| match f {
                    crate::ir::DialectFeature::Rust(rust_dialect_feature) => match rust_dialect_feature {
                        crate::ir::RustDialectFeatures::Wgpu => Some(crate::ir::DialectFeature::Wasm(crate::ir::WasmRuntimeFeatures::Wgpu)),
                    },
                    crate::ir::DialectFeature::Wasm(_) => Some(f.clone()),
                    _ => {
                        log::warn!("Tried mapping feature from unsupported runtime.");
                        requires_unsupported_feature = true;
                        None
                    }
                })
                .collect();

            if !runtime.features().is_superset(&translated_features) || requires_unsupported_feature {
                return None;
            }

            let mut enabled_features = translated_features;
            for feature in runtime.features() {
                if feature.enable_implicitly() {
                    enabled_features.insert(feature.clone());
                }
            }

            Some(enabled_features)
        }
        super::Runtime::NativeBase(_native_runtime, _features) => {
            log::info!("WHY NOT NATIVE");

            if !["RUST", "NATIVE"].contains(&source_format.base_type.as_str()) || disable_native {
                return None;
            }

            let mut requires_unsupported_feature = false;
            let translated_features: std::collections::BTreeSet<_> = source_format
                .features
                .iter()
                .filter_map(|f| match f {
                    crate::ir::DialectFeature::Rust(rust_dialect_feature) => match rust_dialect_feature {
                        crate::ir::RustDialectFeatures::Wgpu => {
                            requires_unsupported_feature = true;
                            None
                        }
                    },
                    crate::ir::DialectFeature::Native(_) => Some(f.clone()),
                    _ => {
                        log::warn!("Tried mapping feature from unsupported runtime.");
                        requires_unsupported_feature = true;
                        None
                    }
                })
                .collect();

            log::info!("{:?} {:?} {}", runtime.features(), translated_features, requires_unsupported_feature);

            if !runtime.features().is_superset(&translated_features) || requires_unsupported_feature {
                return None;
            }

            let mut enabled_features = translated_features;
            for feature in runtime.features() {
                if feature.enable_implicitly() {
                    enabled_features.insert(feature.clone());
                }
            }

            Some(enabled_features)
        }
    }
}
