// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-FileCopyrightText: © 2023 University of Cambridge, System Research Group
// SPDX-FileCopyrightText: © 2024 Roman Kolcun <roman.kolcun@cl.cam.ac.uk>
// SPDX-License-Identifier: MIT

use super::super::*;

pub struct RustCompiler {}

impl RustCompiler {
    pub fn new() -> Self {
        Self {}
    }
}

impl super::StatefulTransformation<crate::ir::support::image_cache::ImageCache> for RustCompiler {
    fn apply(
        &mut self,
        workflow: &mut crate::ir::workflow::ActiveWorkflow,
        nodes: &crate::ir::Nodes,
        _peer_clusters: &crate::ir::Clusters,
        store: &crate::ir::support::image_cache::ImageCache,
    ) {
        for function in workflow.functions.values() {
            let function = function.borrow_mut();
            if function.image.main_image.behavior_image_id.dialect_type.base_type != "RUST" {
                continue;
            }
            for instance in &function.instances {
                let mut instance = instance.borrow_mut();
                if let super::super::PhysicalComponentState::Planned(component) = &mut *instance {
                    let actor_instance = component.as_actor().unwrap();
                    let image_ident = actor::BehaviorImageId {
                        behavior_id: function.image.spec.behavior_id.clone(),
                        dialect_type: actor_instance.runtime_type.clone(),
                        enabled_ports: crate::ir::actor::EnabledPorts {
                            enabled_inputs: function.enabled_inputs().iter().cloned().collect(),
                            enabled_outputs: function.enabled_outputs().iter().cloned().collect(),
                        },
                    };

                    match store.get_blocking(&image_ident) {
                        support::image_cache::CacheResult::NotFound => {
                            build_new_image(&function, actor_instance, image_ident, nodes, store);
                        }
                        support::image_cache::CacheResult::PartialMatch(partial_match) => {
                            let mut image_options: Vec<_> = partial_match
                                .same_runtime_feature_subset(&image_ident)
                                .same_or_more_ports(&image_ident)
                                .into();
                            image_options.sort_by(|a, b| {
                                let a_diff = a
                                    .behavior_image_id
                                    .dialect_type
                                    .features
                                    .difference(&image_ident.dialect_type.features)
                                    .count();
                                let b_diff = b
                                    .behavior_image_id
                                    .dialect_type
                                    .features
                                    .difference(&image_ident.dialect_type.features)
                                    .count();
                                a_diff.cmp(&b_diff)
                            });

                            if let Some(image) = image_options.pop() {
                                actor_instance.image = image;
                            } else {
                                build_new_image(&function, actor_instance, image_ident, nodes, store);
                            }
                        }
                        support::image_cache::CacheResult::FullMatch(actor_image) => actor_instance.image = actor_image,
                    }
                }
            }
        }
    }
}

fn build_new_image(
    logical_instance: &crate::ir::actor::LogicalActor,
    physical_instance: &mut crate::ir::actor::PhysicalActor,
    image_ident: crate::ir::actor::BehaviorImageId,
    nodes: &crate::ir::Nodes,
    store: &crate::ir::support::image_cache::ImageCache,
) {
    let image_result = match physical_instance.runtime_type.base_type.as_str() {
        "WASM" => compile_wasm(logical_instance, image_ident),
        "NATIVE_DYNAMIC" => compile_native(logical_instance, image_ident, *nodes.get(&physical_instance.id.node_id).unwrap()),
        _ => return,
    };

    match image_result {
        Ok(image) => {
            physical_instance.image = image.clone();
            store.insert_blocking(image);
        }
        Err(e) => {
            log::warn!("Failed Compiling Image:\n{e}");
        }
    }
}

fn compile_wasm(actor: &crate::ir::actor::LogicalActor, image_ident: actor::BehaviorImageId) -> Result<actor::BehaviorImage, anyhow::Error> {
    let enabled_features = port_features_for(&image_ident);

    let rust_dir = edgeless_build::rust::unpack_rust_package(&actor.image.main_image.image)?;
    let wasm_file = edgeless_build::wasm::rust_to_wasm(rust_dir, enabled_features, true, false)?;
    let wasm_code = std::fs::read(wasm_file).map_err(|e| anyhow::anyhow!("Cold not read wasm file: {}", e))?;

    Ok(actor::BehaviorImage {
        behavior_image_id: image_ident,
        image: wasm_code,
    })
}

fn compile_native(
    actor: &crate::ir::actor::LogicalActor,
    image_ident: crate::ir::actor::BehaviorImageId,
    node: &dyn crate::ir::Node,
) -> Result<actor::BehaviorImage, anyhow::Error> {
    let enabled_features = port_features_for(&image_ident);

    let rts = node.available_runtimes();
    let rt = rts
        .get("NATIVE_DYNAMIC")
        .ok_or(anyhow::anyhow!("Called native build function on node without native runtime"))?;

    let target = if let Runtime::NativeBase(rt, _features) = rt {
        if rt.node_architecture() == crate::ir::NodeArchitecture::Arm64 {
            edgeless_build::native::NativeTarget::AARCH64
        } else {
            edgeless_build::native::NativeTarget::AMD64
        }
    } else {
        return Err(anyhow::anyhow!("Native Build: Unsupported Target"));
    };

    let features = image_ident
        .dialect_type
        .features
        .iter()
        .filter_map(|feature| match feature {
            DialectFeature::Native(native_runtime_feature) => match native_runtime_feature {
                NativeRuntimeFeatures::Amd64 => {
                    if target != edgeless_build::native::NativeTarget::AMD64 {
                        log::error!("Node architecture does not match Target Architecture Feature")
                    }
                    None
                }
                NativeRuntimeFeatures::Aarch64 => {
                    if target != edgeless_build::native::NativeTarget::AARCH64 {
                        log::error!("Node architecture does not match Target Architecture Feature")
                    }
                    None
                }
                NativeRuntimeFeatures::Aes => Some(edgeless_build::native::NativeFeature::Aes),
            },
            _ => {
                log::error!("Called native build with invalid feature");
                None
            }
        })
        .collect();

    let rust_dir = edgeless_build::rust::unpack_rust_package(&actor.image.main_image.image)?;
    let so_file = edgeless_build::native::rust_to_dynlib(rust_dir, enabled_features, true, false, target, features)?;
    let so_code = std::fs::read(so_file).map_err(|e| anyhow::anyhow!("Cold not read wasm file: {}", e))?;

    Ok(actor::BehaviorImage {
        behavior_image_id: image_ident,
        image: so_code,
    })
}

fn port_features_for(image_ident: &actor::BehaviorImageId) -> Vec<String> {
    let mut enabled_features: Vec<String> = Vec::new();
    for input in &image_ident.enabled_ports.enabled_inputs {
        enabled_features.push(format!("input_{}", input.0))
    }
    for output in &image_ident.enabled_ports.enabled_outputs {
        enabled_features.push(format!("output_{}", output.0))
    }

    enabled_features
}
