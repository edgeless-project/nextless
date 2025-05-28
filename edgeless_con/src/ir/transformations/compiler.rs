// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-FileCopyrightText: © 2023 University of Cambridge, System Research Group
// SPDX-FileCopyrightText: © 2024 Roman Kolcun <roman.kolcun@cl.cam.ac.uk>
// SPDX-License-Identifier: MIT

use super::super::*;

pub struct Compiler {}

impl Compiler {
    pub fn new() -> Self {
        Self {}
    }
}

impl super::StatelessTransformation for Compiler {
    fn apply(&mut self, workflow: &mut crate::ir::workflow::ActiveWorkflow, nodes: &crate::ir::Nodes, _peer_clusters: &crate::ir::Clusters) {
        for function in workflow.functions.values() {
            let function = function.borrow_mut();
            if function.image.format != "RUST" {
                continue;
            }
            for instance in &function.instances {
                if let super::super::PhysicalComponentState::Existing(instance) = &mut *instance.borrow_mut() {
                    if instance.image.is_none() {
                        match instance.runtime_type.as_str() {
                            "WASM_BASE" => compile_wasm(&function, instance),
                            "NATIVE_BASE" => compile_native(&function, instance, *nodes.get(&instance.id.node_id).unwrap()),
                            _ => continue,
                        };
                    }
                }
            }
        }
    }
}

fn compile_wasm(actor: &crate::ir::actor::LogicalActor, instance: &mut crate::ir::actor::PhysicalActor) -> Result<(), anyhow::Error> {
    let enabled_inputs = actor.enabled_inputs();
    let enabled_outputs = actor.enabled_outputs();

    let mut enabled_features: Vec<String> = Vec::new();
    for input in &enabled_inputs {
        enabled_features.push(format!("input_{}", input.0))
    }
    for output in &enabled_outputs {
        enabled_features.push(format!("output_{}", output.0))
    }

    let rust_dir = edgeless_build::unpack_rust_package(&actor.image.code).unwrap();
    let wasm_file = edgeless_build::rust_to_wasm(rust_dir, enabled_features, true, false).unwrap();
    let wasm_code = std::fs::read(wasm_file).unwrap();

    instance.image = Some(actor::ActorImage {
        class: actor.image.class.clone(),
        format: "RUST_WASM".to_string(),
        enabled_inputs: enabled_inputs.iter().cloned().collect(),
        enabled_outputs: enabled_outputs.iter().cloned().collect(),
        code: wasm_code.clone(),
    });

    Ok(())
}

fn compile_native(
    actor: &crate::ir::actor::LogicalActor,
    instance: &mut crate::ir::actor::PhysicalActor,
    node: &dyn crate::ir::Node,
) -> Result<(), anyhow::Error> {
    let enabled_inputs = actor.enabled_inputs();
    let enabled_outputs = actor.enabled_outputs();

    let mut enabled_features: Vec<String> = Vec::new();
    for input in &enabled_inputs {
        enabled_features.push(format!("input_{}", input.0))
    }
    for output in &enabled_outputs {
        enabled_features.push(format!("output_{}", output.0))
    }

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

    instance.image = Some(actor::ActorImage {
        class: actor.image.class.clone(),
        format: "NATIVE_BASE".to_string(),
        enabled_inputs: enabled_inputs.iter().cloned().collect(),
        enabled_outputs: enabled_outputs.iter().cloned().collect(),
        code: so_code.clone(),
    });
    Ok(())
}
