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

impl super::StatelessTransformation for TopicConverter {
    fn apply(&mut self, workflow: &mut crate::ir::workflow::ActiveWorkflow, _nodes: &crate::ir::Nodes, _peer_clusters: &crate::ir::Clusters) {
        let mut targets = std::collections::HashMap::<String, Vec<(String, edgeless_api::function_instance::PortId)>>::new();

        // Find Targets
        for (cid, component) in &mut workflow.components() {
            component
                .borrow_mut()
                .logical_ports()
                .logical_input_mapping
                .retain(|port_id, port_mapping| match port_mapping {
                    LogicalInput::Topic(topic) => {
                        targets.entry(topic.clone()).or_default().push((cid.to_string(), port_id.clone()));
                        false
                    }
                    _ => true,
                })
        }

        // Create Outputs
        for (_cid, component) in &mut workflow.components() {
            let mut component = component.borrow_mut();
            let output_mapping = &mut component.logical_ports().logical_output_mapping;

            *output_mapping = std::mem::take(output_mapping)
                .into_iter()
                .filter_map(|(port_id, port_mapping)| {
                    if let LogicalOutput::Topic(topic) = port_mapping.clone() {
                        if let Some(t) = targets.get(&topic) {
                            return Some((port_id, LogicalOutput::AllOfTargets(t.clone())));
                        }
                    }
                    None
                })
                .collect();
        }
    }
}
