// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

pub struct StaticColocation {}

impl super::FilterStrategy for StaticColocation {
    fn filter_candidates<'b>(
        &mut self,
        logical_component: &dyn crate::ir::LogicalComponent,
        candidates: Vec<crate::ir::transformations::placement::Candidate<'b>>,
        workflow: &crate::ir::workflow::ActiveWorkflow,
    ) -> Vec<crate::ir::transformations::placement::Candidate<'b>> {
        if candidates.is_empty() {
            return vec![];
        }

        let mut logical_peers = std::collections::HashMap::<String, u64>::new();

        {
            for input in logical_component.logical_ports().logical_input_mapping.values() {
                match input {
                    crate::ir::LogicalInput::Direct(items) => {
                        for (item_id, _) in items {
                            *logical_peers.entry(item_id.clone()).or_insert(0) += 1;
                        }
                    }
                    crate::ir::LogicalInput::Topic(_) => {
                        log::warn!("Topic Port occured after logical phase.")
                    }
                }
            }

            for output in logical_component.logical_ports().logical_output_mapping.values() {
                match output {
                    edgeless_api::workflow_instance::PortMapping::DirectTarget(logical_id, _port_id) => {
                        *logical_peers.entry(logical_id.clone()).or_insert(0) += 1;
                    }
                    edgeless_api::workflow_instance::PortMapping::AnyOfTargets(items) => {
                        for (logical_id, _port_id) in items {
                            *logical_peers.entry(logical_id.clone()).or_insert(0) += 1;
                        }
                    }
                    edgeless_api::workflow_instance::PortMapping::AllOfTargets(items) => {
                        for (logical_id, _port_id) in items {
                            *logical_peers.entry(logical_id.clone()).or_insert(0) += 1;
                        }
                    }
                    edgeless_api::workflow_instance::PortMapping::Topic(_) => {
                        log::warn!("Topic Port occured after logical phase.")
                    }
                }
            }
        }

        let mut physical_peers = std::collections::HashMap::<edgeless_api::function_instance::NodeId, u64>::new();

        for (c_id, c) in workflow.components() {
            let logical_link_count = if let Some(link) = logical_peers.get(c_id) {
                link
            } else {
                continue;
            };

            for instance_id in c.borrow_mut().instance_ids() {
                *physical_peers.entry(instance_id.node_id).or_insert(0) += logical_link_count;
            }
        }

        let mut best_count = 0;
        let mut best_peers = std::collections::BTreeSet::new();
        for (peer, link_count) in physical_peers {
            if link_count > best_count {
                best_count = link_count;
                best_peers.clear();
            }
            if link_count == best_count {
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
        StaticColocation {}
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::ir::test::*;
    use crate::ir::transformations::placement::candidate_filter::test_helpers::{mock_function_under_test, mock_peer_function};
    use crate::ir::transformations::placement::candidate_filter::FilterStrategy;

    #[test]
    fn selects_single_qualidfied_candidate() {
        let mut colocation_filter = StaticColocation::new();
        let runtime = MockWasmRuntime {};

        let (node_ids, candidates) = mock_nodes_and_candidates(3, &runtime);
        let colocated_peer_id = edgeless_api::function_instance::InstanceId::new(node_ids[0]);

        let function_under_test = mock_function_under_test(vec![]);
        let other_function = mock_peer_function(vec![colocated_peer_id]);

        let workflow = mock_workflow(std::collections::HashMap::from([
            ("fut".to_string(), function_under_test),
            ("f_other".to_string(), other_function),
        ]));

        let fut_ref = workflow.get_component("fut").unwrap().borrow_mut();

        let filtered = colocation_filter.filter_candidates(&*fut_ref, candidates, &workflow);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered.get(0).unwrap().node_id, colocated_peer_id.node_id);
    }

    #[test]
    fn selects_multiple_qualified_candidates() {
        let mut colocation_filter = StaticColocation::new();
        let runtime = MockWasmRuntime {};

        let (node_ids, candidates) = mock_nodes_and_candidates(3, &runtime);
        let colocated_peer_id = edgeless_api::function_instance::InstanceId::new(node_ids[0]);
        let colocated_peer_id2 = edgeless_api::function_instance::InstanceId::new(node_ids[1]);

        let function_under_test = mock_function_under_test(vec![]);
        let other_function = mock_peer_function(vec![colocated_peer_id, colocated_peer_id2]);

        let workflow = mock_workflow(std::collections::HashMap::from([
            ("fut".to_string(), function_under_test),
            ("f_other".to_string(), other_function),
        ]));

        let fut_ref = workflow.get_component("fut").unwrap().borrow_mut();

        let filtered = colocation_filter.filter_candidates(&*fut_ref, candidates, &workflow);
        assert_eq!(filtered.len(), 2);

        let filtered_candidate_node_ids: std::collections::HashSet<uuid::Uuid> = filtered.into_iter().map(|i| i.node_id).collect();
        let expected_candidate_node_ids = std::collections::HashSet::<uuid::Uuid>::from([node_ids[0].clone(), node_ids[1].clone()]);

        assert_eq!(filtered_candidate_node_ids, expected_candidate_node_ids);
    }

    #[test]
    fn selects_candidate_with_more_linked_instances() {
        let mut colocation_filter = StaticColocation::new();
        let runtime = MockWasmRuntime {};

        let (node_ids, candidates) = mock_nodes_and_candidates(3, &runtime);
        let colocated_peer_id = edgeless_api::function_instance::InstanceId::new(node_ids[0]);
        let colocated_peer_id2 = edgeless_api::function_instance::InstanceId::new(node_ids[0]);
        let colocated_peer_id3 = edgeless_api::function_instance::InstanceId::new(node_ids[1]);

        let function_under_test = mock_function_under_test(vec![]);
        let other_function = mock_peer_function(vec![colocated_peer_id, colocated_peer_id2, colocated_peer_id3]);

        let workflow = mock_workflow(std::collections::HashMap::from([
            ("fut".to_string(), function_under_test),
            ("f_other".to_string(), other_function),
        ]));

        let fut_ref = workflow.get_component("fut").unwrap().borrow_mut();

        let filtered = colocation_filter.filter_candidates(&*fut_ref, candidates, &workflow);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered.get(0).unwrap().node_id, node_ids[0]);
    }

    #[test]
    fn returns_input_when_there_are_no_instances() {
        let mut colocation_filter = StaticColocation::new();
        let runtime = MockWasmRuntime {};

        let (_node_ids, candidates) = mock_nodes_and_candidates(3, &runtime);

        let function_under_test = mock_function_under_test(vec![]);
        let other_function = mock_peer_function(vec![
            edgeless_api::function_instance::InstanceId::new(uuid::Uuid::new_v4()),
            edgeless_api::function_instance::InstanceId::new(uuid::Uuid::new_v4()),
            edgeless_api::function_instance::InstanceId::new(uuid::Uuid::new_v4()),
        ]);

        let workflow = mock_workflow(std::collections::HashMap::from([
            ("fut".to_string(), function_under_test),
            ("f_other".to_string(), other_function),
        ]));

        let fut_ref = workflow.get_component("fut").unwrap().borrow_mut();

        let filtered = colocation_filter.filter_candidates(&*fut_ref, candidates.clone(), &workflow);
        assert_eq!(filtered.len(), 3);

        let filtered_candidate_node_ids: std::collections::HashSet<uuid::Uuid> = filtered.into_iter().map(|i| i.node_id).collect();
        let expected_candidate_node_ids: std::collections::HashSet<uuid::Uuid> = candidates.iter().cloned().map(|i| i.node_id).collect();

        assert_eq!(filtered_candidate_node_ids, expected_candidate_node_ids);
    }
}
