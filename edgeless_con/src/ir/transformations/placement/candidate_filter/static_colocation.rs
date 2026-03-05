// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

pub struct StaticColocation {}

impl super::FilterStrategy for StaticColocation {
    fn filter_candidates<'b>(
        &mut self,
        _logical_component_id: String,
        logical_component: &crate::ir::LogicalComponent,
        candidates: Vec<crate::ir::transformations::placement::Candidate<'b>>,
        workflow: &crate::ir::workflow::ActiveWorkflow,
    ) -> Vec<crate::ir::transformations::placement::Candidate<'b>> {
        if candidates.is_empty() {
            return vec![];
        }

        let port_weights = crate::ir::support::logical_port_scoring::port_weights(logical_component);
        let logical_peer_weights: std::collections::HashMap<_, _> =
            crate::ir::support::logical_port_scoring::peer_weights(logical_component, port_weights)
                .into_iter()
                .map(|(peer, score)| (peer, score as u64))
                .collect();

        let mut physical_peers = std::collections::HashMap::<edgeless_api::function_instance::NodeId, u64>::new();

        for (c_id, _c, c_instances) in workflow.components_with_instances() {
            let logical_peer_weight = if let Some(link) = logical_peer_weights.get(c_id) {
                link
            } else {
                continue;
            };

            for instance_id in c_instances
                .clone()
                .filter_active()
                .filter_map(|physical_instance| physical_instance.component.id())
            {
                *physical_peers.entry(instance_id.node_id).or_insert(0) += logical_peer_weight;
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

        let actor_image = crate::ir::test::mock_actor_image();
        let (node_ids, candidates) = mock_nodes_and_candidates(3, &runtime, actor_image.main_image.behavior_image_id);
        let colocated_peer_id = edgeless_api::function_instance::InstanceId::new(node_ids[0]);

        let (function_under_test, function_under_test_instances) = mock_function_under_test(vec![]);
        let (other_function, other_function_instances) = mock_peer_function(vec![colocated_peer_id]);

        let workflow = crate::ir::workflow::mock_workflow::MockWorkflowBuilder::default()
            .with_component("fut", &function_under_test, function_under_test_instances.as_slice())
            .with_component("f_other", &other_function, other_function_instances.as_slice())
            .build();

        let (fut_ref, _component_instances) = workflow.get_component_with_instances("fut").unwrap();

        let filtered = colocation_filter.filter_candidates("fut".to_string(), &*fut_ref, candidates, &workflow);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered.get(0).unwrap().node_id(), colocated_peer_id.node_id);
    }

    #[test]
    fn selects_multiple_qualified_candidates() {
        let mut colocation_filter = StaticColocation::new();
        let runtime = MockWasmRuntime {};

        let actor_image = crate::ir::test::mock_actor_image();
        let (node_ids, candidates) = mock_nodes_and_candidates(3, &runtime, actor_image.main_image.behavior_image_id);
        let colocated_peer_id = edgeless_api::function_instance::InstanceId::new(node_ids[0]);
        let colocated_peer_id2 = edgeless_api::function_instance::InstanceId::new(node_ids[1]);

        let (function_under_test, function_under_test_instances) = mock_function_under_test(vec![]);
        let (other_function, other_function_instances) = mock_peer_function(vec![colocated_peer_id, colocated_peer_id2]);

        let workflow = crate::ir::workflow::mock_workflow::MockWorkflowBuilder::default()
            .with_component("fut", &function_under_test, function_under_test_instances.as_slice())
            .with_component("f_other", &other_function, other_function_instances.as_slice())
            .build();

        let (fut_ref, _component_instances) = workflow.get_component_with_instances("fut").unwrap();

        let filtered = colocation_filter.filter_candidates("fut".to_string(), &*fut_ref, candidates, &workflow);
        assert_eq!(filtered.len(), 2);

        let filtered_candidate_node_ids: std::collections::HashSet<uuid::Uuid> = filtered.into_iter().map(|i| i.node_id()).collect();
        let expected_candidate_node_ids = std::collections::HashSet::<uuid::Uuid>::from([node_ids[0].clone(), node_ids[1].clone()]);

        assert_eq!(filtered_candidate_node_ids, expected_candidate_node_ids);
    }

    #[test]
    fn selects_candidate_with_more_linked_instances() {
        let mut colocation_filter = StaticColocation::new();
        let runtime = MockWasmRuntime {};

        let actor_image = crate::ir::test::mock_actor_image();
        let (node_ids, candidates) = mock_nodes_and_candidates(3, &runtime, actor_image.main_image.behavior_image_id);
        let colocated_peer_id = edgeless_api::function_instance::InstanceId::new(node_ids[0]);
        let colocated_peer_id2 = edgeless_api::function_instance::InstanceId::new(node_ids[0]);
        let colocated_peer_id3 = edgeless_api::function_instance::InstanceId::new(node_ids[1]);

        let (function_under_test, function_under_test_instances) = mock_function_under_test(vec![]);
        let (other_function, other_function_instances) = mock_peer_function(vec![colocated_peer_id, colocated_peer_id2, colocated_peer_id3]);

        let workflow = crate::ir::workflow::mock_workflow::MockWorkflowBuilder::default()
            .with_component("fut", &function_under_test, function_under_test_instances.as_slice())
            .with_component("f_other", &other_function, other_function_instances.as_slice())
            .build();

        let (fut_ref, _component_instances) = workflow.get_component_with_instances("fut").unwrap();

        let filtered = colocation_filter.filter_candidates("fut".to_string(), &*fut_ref, candidates, &workflow);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered.get(0).unwrap().node_id(), node_ids[0]);
    }

    #[test]
    fn returns_input_when_there_are_no_instances() {
        let mut colocation_filter = StaticColocation::new();
        let runtime = MockWasmRuntime {};

        let actor_image = crate::ir::test::mock_actor_image();
        let (_node_ids, candidates) = mock_nodes_and_candidates(3, &runtime, actor_image.main_image.behavior_image_id);

        let (function_under_test, function_under_test_instances) = mock_function_under_test(vec![]);
        let (other_function, other_function_instances) = mock_peer_function(vec![
            edgeless_api::function_instance::InstanceId::new(uuid::Uuid::new_v4()),
            edgeless_api::function_instance::InstanceId::new(uuid::Uuid::new_v4()),
            edgeless_api::function_instance::InstanceId::new(uuid::Uuid::new_v4()),
        ]);

        let workflow = crate::ir::workflow::mock_workflow::MockWorkflowBuilder::default()
            .with_component("fut", &function_under_test, function_under_test_instances.as_slice())
            .with_component("f_other", &other_function, other_function_instances.as_slice())
            .build();

        let (fut_ref, _component_instances) = workflow.get_component_with_instances("fut").unwrap();

        let filtered = colocation_filter.filter_candidates("fut".to_string(), &*fut_ref, candidates.clone(), &workflow);
        assert_eq!(filtered.len(), 3);

        let filtered_candidate_node_ids: std::collections::HashSet<uuid::Uuid> = filtered.into_iter().map(|i| i.node_id()).collect();
        let expected_candidate_node_ids: std::collections::HashSet<uuid::Uuid> = candidates.iter().cloned().map(|i| i.node_id()).collect();

        assert_eq!(filtered_candidate_node_ids, expected_candidate_node_ids);
    }
}
