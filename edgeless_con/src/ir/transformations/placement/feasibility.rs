// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

pub fn feasible_node_runtime_candidates<'a, 'b>(
    actor: &'a crate::ir::actor::LogicalActor,
    node: &'b dyn crate::ir::Node,
) -> Vec<super::Candidate<'b>> {
    let mut candidates = Vec::new();

    if !node_fulfills_constraints(actor, node) {
        return Vec::new();
    }

    for (_rt_id, rt) in node.available_runtimes() {
        if runtime_supported(actor.image.format.as_str(), &rt) {
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
        if node
            .available_resource_providers()
            .iter()
            .find(|r| r.1.class_type().as_str() == required_resource_class.as_str())
            .is_none()
        {
            return false;
        }
    }

    true
}

fn runtime_supported(code_format: &str, runtime: &crate::ir::Runtime) -> bool {
    match runtime {
        super::Runtime::WasmBase(_wasm_runtime) => vec!["RUST", "RUST_WASM"].contains(&code_format),
        super::Runtime::Native(_native_runtime) => vec!["RUST", "RUST_WASM"].contains(&code_format),
    }
}
