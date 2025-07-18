// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-FileCopyrightText: © 2023 University of Cambridge, System Research Group
// SPDX-FileCopyrightText: © 2024 Roman Kolcun <roman.kolcun@cl.cam.ac.uk>
// SPDX-License-Identifier: MIT

use super::super::*;

pub struct Compiler {}

#[derive(Default)]
pub struct CompilerStore {
    inner: std::sync::Arc<tokio::sync::Mutex<CompilerStoreInner>>,
}

#[derive(Default)]
struct CompilerStoreInner {
    images: std::collections::HashMap<crate::ir::actor::ActorImageIdent, crate::ir::actor::ActorImage>,
}

impl Compiler {
    pub fn new() -> Self {
        Self {}
    }
}

impl super::StatefulTransformation<CompilerStore> for Compiler {
    fn apply(
        &mut self,
        workflow: &mut crate::ir::workflow::ActiveWorkflow,
        nodes: &crate::ir::Nodes,
        _peer_clusters: &crate::ir::Clusters,
        store: &CompilerStore,
    ) {
        for function in workflow.functions.values() {
            let function = function.borrow_mut();
            if function.image.id.format != "RUST" {
                continue;
            }
            for instance in &function.instances {
                if let super::super::PhysicalComponentState::Planned(instance) = &mut *instance.borrow_mut() {
                    let instance = instance.as_actor().unwrap();
                    if instance.image.id.format == "RUST" {
                        let image_ident = actor::ActorImageIdent {
                            class_id: function.image.class.id.clone(),
                            format: match instance.runtime_type.as_str() {
                                "WASM_BASE" => "RUST_WASM".to_string(),
                                _ => instance.runtime_type.clone(),
                            },
                            enabled_inputs: function.enabled_inputs().iter().cloned().collect(),
                            enabled_outputs: function.enabled_outputs().iter().cloned().collect(),
                        };

                        match store.inner.blocking_lock().images.entry(image_ident.clone()) {
                            std::collections::hash_map::Entry::Occupied(occupied_entry) => instance.image = occupied_entry.get().clone(),
                            std::collections::hash_map::Entry::Vacant(vacant_entry) => {
                                let image_result = match instance.runtime_type.as_str() {
                                    "WASM_BASE" => compile_wasm(&function, image_ident),
                                    // TODO: The Architecture needs to be part of the ident.
                                    "NATIVE_BASE" => compile_native(&function, image_ident, *nodes.get(&instance.id.node_id).unwrap()),
                                    _ => continue,
                                };

                                if let Ok(image) = image_result {
                                    instance.image = image.clone();
                                    vacant_entry.insert(image);
                                } else {
                                    log::error!("Failed Compiling Image");
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn compile_wasm(actor: &crate::ir::actor::LogicalActor, image_ident: actor::ActorImageIdent) -> Result<actor::ActorImage, anyhow::Error> {
    let enabled_features = port_features_for(&image_ident);

    let rust_dir = edgeless_build::unpack_rust_package(&actor.image.code).unwrap();
    let wasm_file = edgeless_build::rust_to_wasm(rust_dir, enabled_features, true, false).unwrap();
    let wasm_code = std::fs::read(wasm_file).unwrap();

    Ok(actor::ActorImage {
        class: actor.image.class.clone(),
        id: image_ident,
        code: wasm_code.clone(),
    })
}

fn compile_native(
    actor: &crate::ir::actor::LogicalActor,
    image_ident: crate::ir::actor::ActorImageIdent,
    node: &dyn crate::ir::Node,
) -> Result<actor::ActorImage, anyhow::Error> {
    let enabled_features = port_features_for(&image_ident);

    let rts = node.available_runtimes();
    let rt = rts.get("NATIVE_BASE").ok_or(anyhow::anyhow!("Called native build function on "))?;

    let target = if let Runtime::NativeBase(rt) = rt {
        if rt.node_architecture() == crate::ir::NodeArchitecture::Arm64 {
            edgeless_build::NativeTarget::AARCH64
        } else {
            edgeless_build::NativeTarget::AMD64
        }
    } else {
        return Err(anyhow::anyhow!("Unsupported Target"));
    };

    let rust_dir = edgeless_build::unpack_rust_package(&actor.image.code).unwrap();
    let so_file = edgeless_build::rust_to_dynlib(rust_dir, enabled_features, true, false, target).unwrap();
    let so_code = std::fs::read(so_file).unwrap();

    Ok(actor::ActorImage {
        class: actor.image.class.clone(),
        id: image_ident,
        code: so_code.clone(),
    })
}

fn port_features_for(image_ident: &actor::ActorImageIdent) -> Vec<String> {
    let mut enabled_features: Vec<String> = Vec::new();
    for input in &image_ident.enabled_inputs {
        enabled_features.push(format!("input_{}", input.0))
    }
    for output in &image_ident.enabled_outputs {
        enabled_features.push(format!("output_{}", output.0))
    }

    enabled_features
}
