// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

pub struct DynamicColocation {}

impl super::FilterStrategy for DynamicColocation {
    fn filter_candidates<'b>(
        &mut self,
        logical_component: &dyn crate::ir::LogicalComponent,
        candidates: Vec<crate::ir::transformations::placement::Candidate<'b>>,
        _workflow: &crate::ir::workflow::ActiveWorkflow,
    ) -> Vec<crate::ir::transformations::placement::Candidate<'b>> {
        let mut materialized_instance_count = 0;
        let mut node_rates_abs = std::collections::HashMap::<uuid::Uuid, f64>::new();

        logical_component.instances().iter().for_each(|i| {
            // The borrow will fail for the instance that should currently be placed.
            // This is fine as long as we only use this for new functions but might be
            // probelematic if we use this to check whether the function should be moved.
            if let Ok(maybe_c) = i.try_borrow() {
                if let Some(c) = maybe_c.try_unpack_materialized() {
                    if let Some(materialized) = c.materialized_state() {
                        materialized_instance_count += 1;
                        for input in materialized.borrow_mut().materialized_ports().materialized_inputs.values_mut() {
                            if let Some(port_statistics) = &mut input.port_statistics {
                                for (peer_id, rate) in &port_statistics.message_rate_abs_by_peer(c.creation_time().elapsed()) {
                                    *node_rates_abs.entry(peer_id.node_id).or_insert(0.0) += rate;
                                }
                            }
                        }
                        for output in materialized.borrow_mut().materialized_ports().materialized_outputs.values_mut() {
                            if let Some(port_statistics) = &mut output.port_statistics {
                                for (peer_id, rate) in &port_statistics.message_rate_abs_by_peer(c.creation_time().elapsed()) {
                                    *node_rates_abs.entry(peer_id.node_id).or_insert(0.0) += rate;
                                }
                            }
                        }
                    }
                }
            }
        });

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
            if best_peers.contains(&candidate.node_id) {
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

        let (node_ids, candidates) = mock_nodes_and_candidates(3, &runtime);
        let fut_id = edgeless_api::function_instance::InstanceId::new(node_ids[1]);
        let colocated_peer_id = edgeless_api::function_instance::InstanceId::new(node_ids[0]);

        let mock_stats = Box::new(MockPortStats::new(
            std::collections::HashMap::from([
                (colocated_peer_id.clone(), 10.0),
                (edgeless_api::function_instance::InstanceId::new(node_ids[2]), 1.0),
            ]),
            std::collections::HashMap::new(),
        ));

        let function_under_test =
            crate::ir::transformations::placement::candidate_filter::test_helpers::mock_function_under_test(vec![(fut_id, mock_stats)]);
        let other_function = crate::ir::transformations::placement::candidate_filter::test_helpers::mock_peer_function(vec![colocated_peer_id]);

        let workflow = mock_workflow(std::collections::HashMap::from([
            ("fut".to_string(), function_under_test),
            ("f_other".to_string(), other_function),
        ]));

        let fut_ref = workflow.get_component("fut").unwrap().borrow_mut();

        let filtered = colocation_filter.filter_candidates(&*fut_ref, candidates, &workflow);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered.get(0).unwrap().node_id, colocated_peer_id.node_id);
    }
}
