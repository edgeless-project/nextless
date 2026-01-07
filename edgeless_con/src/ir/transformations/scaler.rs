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
    fn apply(&mut self, workflow: &mut crate::ir::workflow::ActiveWorkflow, _nodes: &crate::ir::Nodes, _peer_clusters: &crate::ir::Clusters) {
        for (logical_function_id, f) in workflow.functions.iter_mut() {
            let mut f = f.borrow_mut();

            let mut active_instance_count = 0;

            for f in &f.instances {
                if f.borrow().try_unpack_active().is_some() {
                    active_instance_count += 1;
                }
            }

            let missing_instance_count = f.constraints.min_instances.unwrap_or(1) - active_instance_count;

            if missing_instance_count > 0 {
                tracing::info!("Function {logical_function_id}: Spawning {missing_instance_count} instances to reach min_instances");
                for _i in 0..missing_instance_count {
                    f.instances
                        .push(std::cell::RefCell::new(super::super::PhysicalComponentState::request_new_instance()));
                }
                continue;
            }

            let can_scale_up = if let Some(max_instances) = f.constraints.max_instances {
                active_instance_count <= max_instances
            } else {
                active_instance_count <= 5
            };

            let mut processing_rate = 0.0;
            let mut message_rate = 0.0;
            let mut last_start: Option<std::time::Instant> = None;

            f.instances.iter().for_each(|i| {
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
                f.instances
                    .push(std::cell::RefCell::new(super::super::PhysicalComponentState::request_new_instance()));
            }

            // if should_scale_down {
            //     f.instances.last().unwrap().borrow_mut().plan_stop();
            // }
        }
        for r in workflow.resources.values_mut() {
            let mut r = r.borrow_mut();

            if r.instances.is_empty() {
                r.instances
                    .push(std::cell::RefCell::new(super::super::PhysicalComponentState::request_new_instance()));
            }
        }
    }
}
