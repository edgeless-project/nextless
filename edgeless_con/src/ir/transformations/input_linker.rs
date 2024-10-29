// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

use super::super::*;

pub struct InputLinker {}

impl InputLinker {
    pub fn new() -> Self {
        Self {}
    }
}

impl super::Transformation for InputLinker {
    fn apply(&mut self, slf: &mut workflow::ActiveWorkflow) {
        let mut inputs = std::collections::HashMap::<
            String,
            std::collections::HashMap<edgeless_api::function_instance::PortId, Vec<(String, edgeless_api::function_instance::PortId)>>,
        >::new();

        for (out_cid, fdesc) in slf.components() {
            for (out_port, mapping) in &fdesc.borrow_mut().logical_ports().logical_output_mapping {
                match mapping {
                    LogicalOutput::DirectTarget(target_fid, target_port) => inputs
                        .entry(target_fid.clone())
                        .or_default()
                        .entry(target_port.clone())
                        .or_default()
                        .push((out_cid.to_string(), out_port.clone())),
                    LogicalOutput::AnyOfTargets(targets) => {
                        for (target_fid, target_port) in targets {
                            inputs
                                .entry(target_fid.clone())
                                .or_default()
                                .entry(target_port.clone())
                                .or_default()
                                .push((out_cid.to_string(), out_port.clone()))
                        }
                    }
                    LogicalOutput::AllOfTargets(targets) => {
                        for (target_fid, target_port) in targets {
                            inputs
                                .entry(target_fid.clone())
                                .or_default()
                                .entry(target_port.clone())
                                .or_default()
                                .push((out_cid.to_string(), out_port.clone()))
                        }
                    }
                    LogicalOutput::Topic(_) => {}
                }
            }
        }

        for (targed_fid, links) in &inputs {
            if let Some(target) = slf.functions.get_mut(targed_fid) {
                for (target_port, sources) in links {
                    target
                        .borrow_mut()
                        .logical_ports
                        .logical_input_mapping
                        .insert(target_port.clone(), LogicalInput::Direct(sources.clone()));
                }
            } else if let Some(target) = slf.resources.get_mut(targed_fid) {
                // Some(&mut target.borrow_mut().ports)
                for (target_port, sources) in links {
                    target
                        .borrow_mut()
                        .logical_ports
                        .logical_input_mapping
                        .insert(target_port.clone(), LogicalInput::Direct(sources.clone()));
                }
            }
        }
    }
}
