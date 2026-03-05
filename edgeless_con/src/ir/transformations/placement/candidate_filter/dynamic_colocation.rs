// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

pub struct DynamicColocation {}

impl super::FilterStrategy for DynamicColocation {
    fn filter_candidates<'b>(
        &mut self,
        logical_component_id: String,
        _logical_component: &crate::ir::LogicalComponent,
        candidates: Vec<crate::ir::transformations::placement::Candidate<'b>>,
        workflow: &crate::ir::workflow::ActiveWorkflow,
    ) -> Vec<crate::ir::transformations::placement::Candidate<'b>> {
        let mut materialized_instance_count = 0;
        let mut node_rates_abs = std::collections::HashMap::<uuid::Uuid, f64>::new();

        let Some((_, component_instances)) = workflow.get_component_with_instances(&logical_component_id) else {
            tracing::warn!("Could not find logical component that should be filtered");
            return candidates;
        };

        for component_instance in component_instances {
            if let Some(c) = component_instance.component.try_unpack_materialized() {
                if let Some(materialized) = c.materialized_state() {
                    materialized_instance_count += 1;
                    for input in materialized.materialized_ports().materialized_inputs.values() {
                        if let Some(port_statistics) = &input.port_statistics {
                            for (peer_id, rate) in &port_statistics.message_rate_abs_by_peer(c.creation_time().elapsed()) {
                                *node_rates_abs.entry(peer_id.node_id).or_insert(0.0) += rate;
                            }
                        }
                    }
                    for output in materialized.materialized_ports().materialized_outputs.values() {
                        if let Some(port_statistics) = &output.port_statistics {
                            for (peer_id, rate) in &port_statistics.message_rate_abs_by_peer(c.creation_time().elapsed()) {
                                *node_rates_abs.entry(peer_id.node_id).or_insert(0.0) += rate;
                            }
                        }
                    }
                }
            }
        }

        if materialized_instance_count == 0 {
            return candidates;
        }

        // Trying to avoid floating point comparisons by reducing this to integer percentages here.
        // The ceil should collect all low-percentage values into the same category.
        let total_port_rate = node_rates_abs.values().cloned().reduce(|acc, e| acc + e).unwrap_or(0.0);
        let node_rates_abs: Vec<_> = node_rates_abs
            .into_iter()
            .map(|(k, v)| {
                assert!(v > 0.0);
                // https://stackoverflow.com/a/37508518
                // https://stackoverflow.com/a/72326090
                (k, (v / total_port_rate * 100.0).ceil() as u64)
            })
            .collect();

        let mut best_rate = 0;
        let mut best_peers = std::collections::BTreeSet::new();

        for (peer, peer_rate) in node_rates_abs {
            if peer_rate > best_rate {
                best_rate = peer_rate;
                best_peers.clear();
            }
            if peer_rate == best_rate {
                best_peers.insert(peer);
            }
        }

        let mut candidate_options = Vec::new();

        for candidate in &candidates {
            if best_peers.contains(&candidate.node_id()) {
                candidate_options.push(candidate.clone())
            }
        }

        if !candidate_options.is_empty() {
            candidate_options
        } else {
            candidates
        }
    }

    fn new() -> Self {
        DynamicColocation {}
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::ir::test::*;
    use crate::ir::transformations::placement::candidate_filter::FilterStrategy;

    #[test]
    fn selects_candidate_with_high_rate_output() {
        let mut colocation_filter = DynamicColocation::new();
        let runtime = MockWasmRuntime {};

        let actor_image = crate::ir::test::mock_actor_image();
        let (node_ids, candidates) = mock_nodes_and_candidates(3, &runtime, actor_image.main_image.behavior_image_id);
        let fut_id = edgeless_api::function_instance::InstanceId::new(node_ids[1]);
        let colocated_peer_id = edgeless_api::function_instance::InstanceId::new(node_ids[0]);

        let mock_stats = Box::new(MockPortStats::new(
            std::collections::HashMap::from([
                (colocated_peer_id.clone(), 10.0),
                (edgeless_api::function_instance::InstanceId::new(node_ids[2]), 1.0),
            ]),
            std::collections::HashMap::new(),
        ));

        let (function_under_test, function_under_test_instances) =
            crate::ir::transformations::placement::candidate_filter::test_helpers::mock_function_under_test(vec![(fut_id, mock_stats)]);
        let (other_function, other_function_instances) =
            crate::ir::transformations::placement::candidate_filter::test_helpers::mock_peer_function(vec![colocated_peer_id]);

        let workflow = crate::ir::workflow::mock_workflow::MockWorkflowBuilder::default()
            .with_component("fut", &function_under_test, function_under_test_instances.as_slice())
            .with_component("f_other", &other_function, other_function_instances.as_slice())
            .build();

        let (fut_ref, _component_instances) = workflow.get_component_with_instances("fut").unwrap();

        let filtered = colocation_filter.filter_candidates("fut".to_string(), &*fut_ref, candidates, &workflow);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered.get(0).unwrap().node_id(), colocated_peer_id.node_id);
    }
}
