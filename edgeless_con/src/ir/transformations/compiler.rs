// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-FileCopyrightText: © 2023 University of Cambridge, System Research Group
// SPDX-FileCopyrightText: © 2024 Roman Kolcun <roman.kolcun@cl.cam.ac.uk>
// SPDX-License-Identifier: MIT

use super::super::*;

pub struct Compiler {}

impl Compiler {
    pub fn new() -> Self {
        Self {}
    }
}

impl super::StatefulPhysicalTransformation<crate::ir::support::image_cache::ImageCache> for Compiler {
    #[tracing::instrument(name = "compiler", skip_all)]
    fn apply(
        &mut self,
        workflow: &crate::ir::workflow::ActiveWorkflow,
        _nodes: &crate::ir::Nodes,
        _peer_clusters: &crate::ir::Clusters,
        store: &crate::ir::support::image_cache::ImageCache,
    ) -> Vec<super::PhysicalChange> {
        let mut required_changes = Vec::new();
        for (_logical_component_id, component, component_instances) in workflow.components_with_instances() {
            let crate::ir::LogicalComponent::Actor(actor) = component else {
                continue;
            };

            for instance in component_instances {
                if let super::super::PhysicalComponentState::Planned(component) = instance.component {
                    let mut actor_component_instance = component.clone();
                    let actor_instance = actor_component_instance.as_actor_mut().unwrap();

                    let image_ident = match &actor_instance.image {
                        actor::ImageState::Planned(behavior_image_id) => behavior_image_id.clone(),
                        actor::ImageState::Existing(_behavior_image) => continue,
                    };

                    let image = match store.get_blocking(&image_ident) {
                        support::image_cache::CacheResult::NotFound => build_new_image(&actor, image_ident, store),
                        support::image_cache::CacheResult::PartialMatch(_partial_match) => {
                            tracing::debug!("Compiler ignoring partially matching image.");
                            build_new_image(&actor, image_ident, store)
                        }
                        support::image_cache::CacheResult::FullMatch(actor_image) => Some(actor_image),
                    };

                    if let Some(image) = image {
                        actor_instance.image = actor::ImageState::Existing(image);

                        let component_update =
                            crate::ir::transformations::PhysicalChange::Component(crate::ir::transformations::PhysicalComponentChange {
                                component_id: instance.component_id,
                                action: crate::ir::transformations::PhysicalComponentChangeAction::Update(
                                    super::super::PhysicalComponentState::Planned(actor_component_instance),
                                ),
                            });

                        required_changes.push(component_update);
                    }
                }
            }
        }
        required_changes
    }

    fn apply_stop(
        &mut self,
        _workflow: &crate::ir::workflow::ActiveWorkflow,
        _nodes: &crate::ir::Nodes,
        _peer_clusters: &crate::ir::Clusters,
        _global_state: &crate::ir::support::image_cache::ImageCache,
    ) -> Vec<transformations::PhysicalChange> {
        // We might want to clear images from the cache here (would require ref-counting.)
        // Probably some other strategy to keep the cache size limited is better here.
        vec![]
    }
}

fn build_new_image(
    logical_instance: &crate::ir::actor::LogicalActor,
    image_ident: crate::ir::behavior::BehaviorImageId,
    store: &crate::ir::support::image_cache::ImageCache,
) -> Option<crate::ir::behavior::BehaviorImage> {
    let image_result = crate::ir::behavior::dialect::DialectRegistry::new_default().try_translate(&logical_instance.image.main_image, &image_ident);

    match image_result {
        Ok(image) => {
            store.insert_blocking(image.clone());
            return Some(image);
        }
        Err(e) => {
            tracing::warn!("Failed Compiling Image:\n{e}");
            None
        }
    }
}
