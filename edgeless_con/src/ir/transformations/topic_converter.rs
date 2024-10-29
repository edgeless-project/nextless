// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

use super::super::*;

pub struct TopicConverter {}

impl TopicConverter {
    pub fn new() -> Self {
        Self {}
    }
}

impl super::Transformation for TopicConverter {
    fn apply(&mut self, workflow: &mut workflow::ActiveWorkflow) {
        let mut targets = std::collections::HashMap::<String, Vec<(String, edgeless_api::function_instance::PortId)>>::new();

        // Find Targets
        for (cid, component) in &mut workflow.components() {
            component
                .borrow_mut()
                .logical_ports()
                .logical_input_mapping
                .retain(|port_id, port_mapping| match port_mapping {
                    LogicalInput::Topic(topic) => {
                        targets
                            .entry(topic.clone())
                            .or_insert(Vec::new())
                            .push((cid.to_string(), port_id.clone()));
                        false
                    }
                    _ => true,
                })
        }

        // Create Outputs
        for (_cid, component) in &mut workflow.components() {
            component
                .borrow_mut()
                .logical_ports()
                .logical_output_mapping
                .iter_mut()
                .for_each(|(_port_id, port_mapping)| {
                    if let LogicalOutput::Topic(topic) = port_mapping.clone() {
                        *port_mapping = LogicalOutput::AllOfTargets(
                            targets
                                .get(&topic)
                                .unwrap_or(&Vec::<(String, edgeless_api::function_instance::PortId)>::new())
                                .clone(),
                        );
                    }
                });
        }
    }
}
