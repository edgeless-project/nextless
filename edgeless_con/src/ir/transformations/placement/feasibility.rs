// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

pub fn feasible_node_runtime_candidates<'b>(
    node_filter: &crate::ir::component::NodeFilters,
    logical_actor: &crate::ir::actor::LogicalActor,
    node: &'b dyn crate::ir::Node,
    allow_suboptimal: bool,
    disable_actor_optimization: bool,
) -> Vec<super::Candidate<'b>> {
    let mut candidates = Vec::new();

    if !node_fulfills_constraints(node_filter, node) {
        return Vec::new();
    }

    for (_rt_id, rt) in node.available_runtimes() {
        let dest_dialect = rt.supported_dialect();

        if let Some(allowed_dialects) = &node_filter.runtime_dialects_allowed {
            if !allowed_dialects.contains(&dest_dialect.base_type) {
                continue;
            }
        }

        if let Some(denied_dialects) = &node_filter.runtime_dialects_denied {
            if denied_dialects.contains(&dest_dialect.base_type) {
                continue;
            }
        }

        let enabled_ports = if disable_actor_optimization {
            logical_actor.image.main_image.behavior_image_id.enabled_ports.clone()
        } else {
            crate::ir::behavior::EnabledPorts {
                enabled_inputs: logical_actor.enabled_inputs().iter().cloned().collect(),
                enabled_outputs: logical_actor.enabled_outputs().iter().cloned().collect(),
            }
        };

        let dest_spec = crate::ir::behavior::BehaviorImageId {
            behavior_id: logical_actor.image.main_image.behavior_image_id.behavior_id.clone(),
            enabled_ports,
            dialect_type: dest_dialect,
        };

        let target_image_id = crate::ir::behavior::dialect::DialectRegistry::new_default()
            .plan_translation(&logical_actor.image.main_image.behavior_image_id, &dest_spec, !allow_suboptimal)
            .ok();

        if let Some(dest_image_id) = target_image_id {
            candidates.push(super::Candidate {
                node_id: node.node_id(),
                runtime: rt.clone(),
                dest_image: crate::ir::actor::ImageState::Planned(dest_image_id),
            });
        }
    }

    candidates
}

fn node_fulfills_constraints(node_filter: &crate::ir::component::NodeFilters, node: &dyn crate::ir::Node) -> bool {
    if let Some(allowed_nodes) = &node_filter.node_ids_allowed {
        if !allowed_nodes.contains(&node.node_id()) {
            return false;
        }
    }

    if let Some(denied_nodes) = &node_filter.node_ids_denied {
        if denied_nodes.contains(&node.node_id()) {
            return false;
        }
    }

    if let Some(allowed_clusters) = &node_filter.cluster_ids_allowed {
        if !allowed_clusters.contains(&node.cluster_id()) {
            return false;
        }
    }

    if let Some(denied_clusters) = &node_filter.cluster_ids_denied {
        if denied_clusters.contains(&node.cluster_id()) {
            return false;
        }
    }

    let node_labels: std::collections::HashSet<String> = node.labels().iter().cloned().collect();

    if let Some(label_sets) = &node_filter.node_label_filter_allowed {
        let mut found_accepted_set = false;

        for label_combination in label_sets {
            if node_labels.is_superset(&label_combination) {
                found_accepted_set = true;
                break;
            }
        }

        if !found_accepted_set {
            return false;
        }
    }

    if let Some(label_sets) = &node_filter.node_label_filter_denied {
        for label_combination in label_sets {
            if node_labels.is_superset(&label_combination) {
                return false;
            }
        }
    }

    true
}

// Common test utilities
#[cfg(test)]
mod test {
    use std::str::FromStr;
    pub struct MockNode {}

    impl crate::ir::Node for MockNode {
        fn node_id(&self) -> edgeless_api::function_instance::NodeId {
            uuid::Uuid::from_str("00000000-0000-0000-0000-000000000001").unwrap()
        }

        fn cluster_id(&self) -> edgeless_api::function_instance::NodeId {
            uuid::Uuid::from_str("00000000-0000-0000-0000-000000000002").unwrap()
        }

        fn available_runtimes<'a>(&'a self) -> crate::ir::Runtimes<'a> {
            std::collections::HashMap::new()
        }

        fn available_resource_providers<'a>(&'a self) -> crate::ir::ResourceProviders<'a> {
            std::collections::HashMap::new()
        }

        fn available_link_types(&self) -> crate::ir::LinkProviders {
            std::collections::HashMap::new()
        }

        fn available_interaction_dialects(&self) -> Vec<crate::ir::interaction::dialect::DialectDescriptor> {
            vec![]
        }

        fn labels(&self) -> Vec<String> {
            vec!["label1".to_string(), "label2".to_string()]
        }

        fn is_proxy(&self) -> bool {
            false
        }
    }
}

#[cfg(test)]
mod constraint_test {
    use super::test::MockNode;
    use std::str::FromStr;

    #[test]
    fn allowed_without_filters() {
        let filter = crate::ir::component::NodeFilters::default();
        let node = MockNode {};
        assert!(super::node_fulfills_constraints(&filter, &node))
    }

    #[test]
    fn allowed_allowed_node_ids() {
        let filter = crate::ir::component::NodeFilters {
            node_ids_allowed: Some(vec![uuid::Uuid::from_str("00000000-0000-0000-0000-000000000001").unwrap()]),
            ..Default::default()
        };
        let node = MockNode {};
        assert!(super::node_fulfills_constraints(&filter, &node))
    }

    #[test]
    fn denied_allowed_node_ids() {
        let filter = crate::ir::component::NodeFilters {
            node_ids_allowed: Some(vec![uuid::Uuid::from_str("00000000-0000-0000-0000-000000000010").unwrap()]),
            ..Default::default()
        };
        let node = MockNode {};
        assert!(super::node_fulfills_constraints(&filter, &node) == false)
    }

    #[test]
    fn allowed_denied_node_ids() {
        let filter = crate::ir::component::NodeFilters {
            node_ids_denied: Some(vec![uuid::Uuid::from_str("00000000-0000-0000-0000-000000000010").unwrap()]),
            ..Default::default()
        };
        let node = MockNode {};
        assert!(super::node_fulfills_constraints(&filter, &node))
    }

    #[test]
    fn denied_denied_node_ids() {
        let filter = crate::ir::component::NodeFilters {
            node_ids_denied: Some(vec![uuid::Uuid::from_str("00000000-0000-0000-0000-000000000001").unwrap()]),
            ..Default::default()
        };
        let node = MockNode {};
        assert!(super::node_fulfills_constraints(&filter, &node) == false)
    }

    #[test]
    fn denied_allowed_and_denied_node_ids() {
        let filter = crate::ir::component::NodeFilters {
            node_ids_allowed: Some(vec![uuid::Uuid::from_str("00000000-0000-0000-0000-000000000001").unwrap()]),
            node_ids_denied: Some(vec![uuid::Uuid::from_str("00000000-0000-0000-0000-000000000001").unwrap()]),
            ..Default::default()
        };
        let node = MockNode {};
        assert!(super::node_fulfills_constraints(&filter, &node) == false)
    }

    #[test]
    fn allowed_allowed_cluster_ids() {
        let filter = crate::ir::component::NodeFilters {
            cluster_ids_allowed: Some(vec![uuid::Uuid::from_str("00000000-0000-0000-0000-000000000002").unwrap()]),
            ..Default::default()
        };
        let node = MockNode {};
        assert!(super::node_fulfills_constraints(&filter, &node))
    }

    #[test]
    fn denied_allowed_cluster_ids() {
        let filter = crate::ir::component::NodeFilters {
            cluster_ids_allowed: Some(vec![uuid::Uuid::from_str("00000000-0000-0000-0000-000000000010").unwrap()]),
            ..Default::default()
        };
        let node = MockNode {};
        assert!(super::node_fulfills_constraints(&filter, &node) == false)
    }

    #[test]
    fn allowed_denied_cluster_ids() {
        let filter = crate::ir::component::NodeFilters {
            cluster_ids_denied: Some(vec![uuid::Uuid::from_str("00000000-0000-0000-0000-000000000010").unwrap()]),
            ..Default::default()
        };
        let node = MockNode {};
        assert!(super::node_fulfills_constraints(&filter, &node))
    }

    #[test]
    fn denied_denied_cluster_ids() {
        let filter = crate::ir::component::NodeFilters {
            cluster_ids_denied: Some(vec![uuid::Uuid::from_str("00000000-0000-0000-0000-000000000002").unwrap()]),
            ..Default::default()
        };
        let node = MockNode {};
        assert!(super::node_fulfills_constraints(&filter, &node) == false)
    }

    #[test]
    fn denied_allowed_and_denied_cluster_ids() {
        let filter = crate::ir::component::NodeFilters {
            cluster_ids_allowed: Some(vec![uuid::Uuid::from_str("00000000-0000-0000-0000-000000000002").unwrap()]),
            cluster_ids_denied: Some(vec![uuid::Uuid::from_str("00000000-0000-0000-0000-000000000002").unwrap()]),
            ..Default::default()
        };
        let node = MockNode {};
        assert!(super::node_fulfills_constraints(&filter, &node) == false)
    }

    #[test]
    fn allowed_allowed_one_label() {
        let filter = crate::ir::component::NodeFilters {
            node_label_filter_allowed: Some(vec![std::collections::HashSet::from(["label1".to_string()])]),
            ..Default::default()
        };
        let node = MockNode {};
        assert!(super::node_fulfills_constraints(&filter, &node))
    }

    #[test]
    fn allowed_allowed_two_labels() {
        let filter = crate::ir::component::NodeFilters {
            node_label_filter_allowed: Some(vec![std::collections::HashSet::from(["label1".to_string(), "label2".to_string()])]),
            ..Default::default()
        };
        let node = MockNode {};
        assert!(super::node_fulfills_constraints(&filter, &node))
    }

    #[test]
    fn allowed_allowed_two_groups() {
        let filter = crate::ir::component::NodeFilters {
            node_label_filter_allowed: Some(vec![
                std::collections::HashSet::from(["label3".to_string()]),
                std::collections::HashSet::from(["label1".to_string(), "label2".to_string()]),
            ]),
            ..Default::default()
        };
        let node = MockNode {};
        assert!(super::node_fulfills_constraints(&filter, &node))
    }

    #[test]
    fn denied_allowed_one_label() {
        let filter = crate::ir::component::NodeFilters {
            node_label_filter_allowed: Some(vec![std::collections::HashSet::from(["label3".to_string()])]),
            ..Default::default()
        };
        let node = MockNode {};
        assert!(super::node_fulfills_constraints(&filter, &node) == false)
    }

    #[test]
    fn denied_allowed_two_labels() {
        let filter = crate::ir::component::NodeFilters {
            node_label_filter_allowed: Some(vec![std::collections::HashSet::from(["label1".to_string(), "label3".to_string()])]),
            ..Default::default()
        };
        let node = MockNode {};
        assert!(super::node_fulfills_constraints(&filter, &node) == false)
    }

    #[test]
    fn allowed_denied_one_label() {
        let filter = crate::ir::component::NodeFilters {
            node_label_filter_denied: Some(vec![std::collections::HashSet::from(["label3".to_string()])]),
            ..Default::default()
        };
        let node = MockNode {};
        assert!(super::node_fulfills_constraints(&filter, &node))
    }

    #[test]
    fn allowed_denied_two_labels() {
        let filter = crate::ir::component::NodeFilters {
            node_label_filter_denied: Some(vec![std::collections::HashSet::from(["label1".to_string(), "label3".to_string()])]),
            ..Default::default()
        };
        let node = MockNode {};
        assert!(super::node_fulfills_constraints(&filter, &node))
    }

    #[test]
    fn denied_denied_one_label() {
        let filter = crate::ir::component::NodeFilters {
            node_label_filter_denied: Some(vec![std::collections::HashSet::from(["label2".to_string()])]),
            ..Default::default()
        };
        let node = MockNode {};
        assert!(super::node_fulfills_constraints(&filter, &node) == false)
    }

    #[test]
    fn denied_denied_two_labels() {
        let filter = crate::ir::component::NodeFilters {
            node_label_filter_denied: Some(vec![std::collections::HashSet::from(["label2".to_string(), "label1".to_string()])]),
            ..Default::default()
        };
        let node = MockNode {};
        assert!(super::node_fulfills_constraints(&filter, &node) == false)
    }

    #[test]
    fn denied_denied_two_groups() {
        let filter = crate::ir::component::NodeFilters {
            node_label_filter_denied: Some(vec![
                std::collections::HashSet::from(["label2".to_string(), "label3".to_string()]),
                std::collections::HashSet::from(["label1".to_string()]),
            ]),
            ..Default::default()
        };
        let node = MockNode {};
        assert!(super::node_fulfills_constraints(&filter, &node) == false)
    }

    #[test]
    fn denied_allowed_and_denied_label() {
        let filter = crate::ir::component::NodeFilters {
            node_label_filter_allowed: Some(vec![std::collections::HashSet::from(["label2".to_string()])]),
            node_label_filter_denied: Some(vec![std::collections::HashSet::from(["label2".to_string()])]),
            ..Default::default()
        };
        let node = MockNode {};
        assert!(super::node_fulfills_constraints(&filter, &node) == false)
    }
}
