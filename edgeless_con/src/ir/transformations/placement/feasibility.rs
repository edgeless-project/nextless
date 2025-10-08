// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

use std::str::FromStr;

pub fn feasible_node_runtime_candidates<'b>(
    actor: &crate::ir::actor::LogicalActor,
    node: &'b dyn crate::ir::Node,
    new_instance: bool,
    allow_suboptimal: bool,
) -> Vec<super::Candidate<'b>> {
    let mut candidates = Vec::new();

    if !node_fulfills_constraints(actor, node) {
        return Vec::new();
    }

    if let Some(dest_node) = actor.annotations.get("node_id_init_on") {
        if actor.instances.len() == 1 && new_instance {
            let dest_uuid = uuid::Uuid::from_str(dest_node).unwrap();
            if dest_uuid != node.node_id() {
                return Vec::new();
            }
        }
    }

    for (_rt_id, rt) in node.available_runtimes() {
        let disable_native = actor.annotations.contains_key("NO_NATIVE");
        let dest_dialect = rt.supported_dialect();

        // This exists for debugging/evaluation puposes.
        // If we need this permanently, this should be automatic and available for all features.
        if disable_native && dest_dialect.base_type == crate::ir::behavior::dialect::native_dyanamic::ID {
            continue;
        }

        let dest_spec = crate::ir::behavior::BehaviorImageId {
            behavior_id: actor.image.main_image.behavior_image_id.behavior_id.clone(),
            enabled_ports: crate::ir::behavior::EnabledPorts {
                enabled_inputs: actor.enabled_inputs().iter().cloned().collect(),
                enabled_outputs: actor.enabled_outputs().iter().cloned().collect(),
            },
            dialect_type: dest_dialect,
        };

        let target_image_id = crate::ir::behavior::dialect::DialectRegistry::new_default()
            .plan_translation(&actor.image.main_image.behavior_image_id.clone(), &dest_spec, !allow_suboptimal)
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

fn node_fulfills_constraints(actor: &crate::ir::actor::LogicalActor, node: &dyn crate::ir::Node) -> bool {
    if let Some(viable_domains) = &actor.constraints.domain_id_match_any {
        if !viable_domains.contains(&node.cluster_id()) {
            return false;
        }
    }

    if let Some(viable_nodes) = &actor.constraints.node_id_match_any {
        if !viable_nodes.contains(&node.node_id()) {
            return false;
        }
    }

    for label in actor.constraints.label_match_all.iter() {
        if !node.labels().contains(label) {
            return false;
        }
    }

    for required_resource_class in &actor.constraints.resource_match_all {
        if !node
            .available_resource_providers()
            .iter()
            .any(|r| r.1.class_type().as_str() == required_resource_class.as_str())
        {
            return false;
        }
    }

    true
}
