// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

pub struct Scaler {}

impl Scaler {
    pub fn new() -> Self {
        Self {}
    }
}

impl super::StatelessTransformation for Scaler {
    fn apply(&mut self, workflow: &mut crate::ir::workflow::ActiveWorkflow, _nodes: &crate::ir::Nodes, _peer_clusters: &crate::ir::Clusters) {
        for (_fid, f) in &mut workflow.functions {
            let mut f = f.borrow_mut();

            let number_of_existing_instances = f.instances.iter().fold(0, |acc, i| match &*i.borrow() {
                super::super::PhysicalComponentState::Existing(_) => acc + 1,
                _ => acc,
            });

            if f.instances.len() == 0 {
                f.instances.push(std::cell::RefCell::new(super::super::PhysicalComponentState::new()));
            }

            let can_scale_up = if let Some(max_instances) = f.constraints.max_instances {
                f.instances.len() < max_instances
            } else {
                f.instances.len() < 5
            };

            let mut processing_rate = 0.0;
            let mut message_rate = 0.0;
            let mut last_start: Option<std::time::Instant> = None;

            f.instances.iter().for_each(|i| {
                if let super::super::PhysicalComponentState::Existing(instance) = &*i.borrow() {
                    if let Some(l) = last_start {
                        last_start = Some(l.max(instance.creation_tine))
                    } else {
                        last_start = Some(instance.creation_tine)
                    }
                    if let Some(materialized) = &instance.materialized {
                        if let Some(rt) = &materialized.borrow_mut().runtime_statistics {
                            let instance_rate = rt.invocation_rate_abs(std::time::Duration::from_secs(60)).unwrap_or(0f64);
                            processing_rate += instance_rate;
                        }

                        for (_pid, p) in &mut materialized.borrow_mut().mapping.materialized_inputs {
                            if let Some(stats) = &p.port_statistics {
                                message_rate += stats.message_rate_abs(std::time::Duration::from_secs(60)).unwrap_or(0f64)
                            }
                        }
                    }
                }
            });

            let should_scale_up = message_rate > 1.1 * processing_rate;
            let should_scale_down = processing_rate < number_of_existing_instances as f64;
            let wait_period_exceeded = if let Some(last_start) = last_start {
                last_start.elapsed() > std::time::Duration::from_secs(60)
            } else {
                false
            };

            if can_scale_up && should_scale_up && wait_period_exceeded {
                log::info!(
                    "Attempting to Scale Up. Message Rate:{} Processing Rate:{}",
                    message_rate,
                    processing_rate
                );
                f.instances.push(std::cell::RefCell::new(super::super::PhysicalComponentState::new()));
            }

            // if should_scale_down {
            //     f.instances.last().unwrap().borrow_mut().plan_stop();
            // }
        }
        for (_rid, r) in &mut workflow.resources {
            let mut r = r.borrow_mut();

            let number_of_existing_instances = r.instances.iter().fold(0, |acc, i| match &*i.borrow() {
                super::super::PhysicalComponentState::Existing(_) => acc + 1,
                _ => acc,
            });

            if r.instances.len() == 0 {
                r.instances.push(std::cell::RefCell::new(super::super::PhysicalComponentState::new()));
            }
        }
    }
}
