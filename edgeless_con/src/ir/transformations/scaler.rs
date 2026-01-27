// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

pub struct Scaler {}

impl Scaler {
    pub fn new() -> Self {
        Self {}
    }
}

const UPDATE_RATE_SECS: u64 = 30;

// Probably should only be called "LoadScaler"
impl super::StatelessTransformation for Scaler {
    #[tracing::instrument(name = "scaler", skip_all)]
    fn apply(
        &mut self,
        workflow: &mut crate::ir::workflow::ActiveWorkflow,
        available_nodes: &crate::ir::Nodes,
        _peer_clusters: &crate::ir::Clusters,
    ) {
        for (logical_component_id, component) in workflow.components() {
            // Skipped as this is not fully implemented yet.
            if logical_component_id == "__proxy" {
                continue;
            }

            let mut component = component.borrow_mut();

            match component.scaling_mode() {
                crate::ir::component::ScalingMode::Singleton { .. } => {
                    scale_singleton(logical_component_id, &mut *component);
                }
                crate::ir::component::ScalingMode::Scalable { .. } => {
                    scale_scalable(logical_component_id, &mut *component);
                }
                crate::ir::component::ScalingMode::AllNodes { .. } => {
                    scale_to_all_nodes(logical_component_id, &mut *component, available_nodes);
                }
            }
        }
    }
}

fn scale_to_all_nodes(logical_function_id: &str, f: &mut dyn crate::ir::logical_model::LogicalComponent, available_nodes: &crate::ir::Nodes) {
    let mut covered_nodes = std::collections::HashSet::<uuid::Uuid>::new();

    for component_instance in &f.instances() {
        let borrowed_instance = component_instance.borrow();

        if let Some(active_actor) = borrowed_instance.try_unpack_active() {
            covered_nodes.insert(active_actor.id().node_id.clone());
        }
    }

    let possible_nodes = f
        .node_filters()
        .node_ids_allowed
        .as_ref()
        .map(|node_ids| node_ids.iter().cloned().collect::<std::collections::HashSet<_>>());

    let available_node_ids = available_nodes.keys().cloned().collect::<std::collections::HashSet<_>>();

    let missing_nodes = available_node_ids.difference(&covered_nodes).filter(|missing_node| {
        if let Some(possible_nodes) = &possible_nodes {
            possible_nodes.contains(missing_node)
        } else {
            true
        }
    });

    for missing_node in missing_nodes {
        tracing::info!("AllNode Actor {logical_function_id}: Spawning instance cover node {missing_node} ");

        let mut node_filter = f.node_filters();
        node_filter.node_ids_allowed = Some(vec![missing_node.clone()]);

        f.instances_mut().push(std::cell::RefCell::new(
            super::super::PhysicalComponentState::request_new_instance_with_extra_constraints(node_filter),
        ));
    }
}

fn scale_singleton(logical_function_id: &str, f: &mut dyn crate::ir::logical_model::LogicalComponent) {
    for f in &f.instances() {
        if f.borrow().try_unpack_active().is_some() {
            return;
        }
    }

    tracing::info!("Singleton Actor {logical_function_id}: Spawning missing instance");

    f.instances_mut()
        .push(std::cell::RefCell::new(super::super::PhysicalComponentState::request_new_instance()));
}

fn scale_scalable(logical_function_id: &str, f: &mut dyn crate::ir::logical_model::LogicalComponent) {
    let crate::ir::component::ScalingMode::Scalable {
        min_instances,
        max_instances,
        ..
    } = &f.scaling_mode()
    else {
        return;
    };

    let mut active_instance_count = 0;

    for f in &f.instances() {
        if f.borrow().try_unpack_active().is_some() {
            active_instance_count += 1;
        }
    }

    let missing_instance_count = if *min_instances > active_instance_count {
        min_instances - active_instance_count
    } else {
        0
    };

    if missing_instance_count > 0 {
        tracing::info!("Scalable Actor {logical_function_id}: Spawning {missing_instance_count} instances to reach min_instances");
        for _i in 0..missing_instance_count {
            f.instances_mut()
                .push(std::cell::RefCell::new(super::super::PhysicalComponentState::request_new_instance()));
        }
        return;
    }

    let can_scale_up = active_instance_count <= *max_instances;

    let mut processing_rate = 0.0;
    let mut message_rate = 0.0;
    let mut last_start: Option<std::time::Instant> = None;

    f.instances_mut().iter().for_each(|i| {
        if let super::super::PhysicalComponentState::Materialized(instance) = &*i.borrow() {
            if let Some(l) = last_start {
                last_start = Some(l.max(instance.creation_time()))
            } else {
                last_start = Some(instance.creation_time())
            }
            if let Some(materialized) = &instance.materialized_state() {
                if let Some(rt) = &materialized.borrow_mut().runtime_statistics() {
                    let instance_rate = rt.invocation_rate_abs(std::time::Duration::from_secs(UPDATE_RATE_SECS)).unwrap_or(0f64);
                    processing_rate += instance_rate;
                }

                for p in materialized.borrow_mut().materialized_ports().materialized_inputs.values_mut() {
                    if let Some(stats) = &p.port_statistics {
                        message_rate += stats.message_rate_abs(std::time::Duration::from_secs(UPDATE_RATE_SECS)).unwrap_or(0f64)
                    }
                }
            }
        }
    });

    tracing::trace!("Function {logical_function_id}: Processing Rate: {processing_rate}, Message Rate: {message_rate}");

    let should_scale_up = message_rate > 1.1 * processing_rate;

    // let should_scale_down = processing_rate < number_of_existing_instances as f64;
    let wait_period_exceeded = if let Some(last_start) = last_start {
        last_start.elapsed() > std::time::Duration::from_secs(UPDATE_RATE_SECS)
    } else {
        false
    };

    if can_scale_up && should_scale_up && wait_period_exceeded {
        tracing::info!("Attempting to Scale Up. Message Rate:{message_rate} Processing Rate:{processing_rate}");
        f.instances_mut()
            .push(std::cell::RefCell::new(super::super::PhysicalComponentState::request_new_instance()));
    }

    // if should_scale_down {
    //     f.instances.last().unwrap().borrow_mut().plan_stop();
    // }
}
