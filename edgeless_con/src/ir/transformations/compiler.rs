// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

use super::super::*;

pub struct Compiler {}

impl Compiler {
    pub fn new() -> Self {
        Self {}
    }
}

impl super::Transformation for Compiler {
    fn apply(&mut self, workflow: &mut crate::ir::workflow::ActiveWorkflow) {
        for (_, function) in &mut workflow.functions {
            let function = function.borrow_mut();
            if function.image.format == "RUST" {
                let enabled_inputs = function.enabled_inputs();
                let enabled_outputs = function.enabled_outputs();

                let mut enabled_features: Vec<String> = Vec::new();
                for input in &enabled_inputs {
                    enabled_features.push(format!("input_{}", input.0))
                }
                for output in &enabled_outputs {
                    enabled_features.push(format!("output_{}", output.0))
                }

                let rust_dir = edgeless_build::unpack_rust_package(&function.image.code).unwrap();
                let wasm_file = edgeless_build::rust_to_wasm(rust_dir, enabled_features, true, false).unwrap();
                let wasm_code = std::fs::read(wasm_file).unwrap();

                for instance in &function.instances {
                    instance.borrow_mut().image = Some(actor::ActorImage {
                        class: function.image.class.clone(),
                        format: "RUST_WASM".to_string(),
                        enabled_inputs: enabled_inputs.iter().cloned().collect(),
                        enabled_outputs: enabled_outputs.iter().cloned().collect(),
                        code: wasm_code.clone(),
                    })
                }
            }
        }
    }
}
