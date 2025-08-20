// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

use std::str::FromStr;

pub fn feasible_node_runtime_candidates<'b>(
    actor: &crate::ir::actor::LogicalActor,
    node: &'b dyn crate::ir::Node,
    new_instance: bool,
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
        if runtime_supported(actor.image.id.format.as_str(), &rt, actor.annotations.get("NO_NATIVE").is_some()) {
            candidates.push(super::Candidate {
                node_id: node.node_id(),
                runtime: rt.clone(),
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

fn runtime_supported(code_format: &str, runtime: &crate::ir::Runtime, disable_native: bool) -> bool {
    match runtime {
        super::Runtime::WasmBase(_wasm_runtime) => ["RUST", "RUST_WASM"].contains(&code_format),
        super::Runtime::NativeBase(_native_runtime) => ["RUST"].contains(&code_format) && !disable_native,
    }
}
