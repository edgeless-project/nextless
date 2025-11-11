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

impl super::StatefulTransformation<crate::ir::support::image_cache::ImageCache> for Compiler {
    fn apply(
        &mut self,
        workflow: &mut crate::ir::workflow::ActiveWorkflow,
        _nodes: &crate::ir::Nodes,
        _peer_clusters: &crate::ir::Clusters,
        store: &crate::ir::support::image_cache::ImageCache,
    ) {
        for function in workflow.functions.values() {
            let function = function.borrow_mut();
            for instance in &function.instances {
                let mut instance = instance.borrow_mut();
                if let super::super::PhysicalComponentState::Planned(component) = &mut *instance {
                    let actor_instance = component.as_actor().unwrap();

                    let image_ident = match &actor_instance.image {
                        actor::ImageState::Planned(behavior_image_id) => behavior_image_id.clone(),
                        actor::ImageState::Existing(_behavior_image) => continue,
                    };

                    match store.get_blocking(&image_ident) {
                        support::image_cache::CacheResult::NotFound => {
                            build_new_image(&function, actor_instance, image_ident, store);
                        }
                        support::image_cache::CacheResult::PartialMatch(_partial_match) => {
                            log::info!("Compile Ignoring Partial Match for Image.");
                            build_new_image(&function, actor_instance, image_ident, store);
                        }
                        support::image_cache::CacheResult::FullMatch(actor_image) => actor_instance.image = actor::ImageState::Existing(actor_image),
                    }
                }
            }
        }
    }
}

fn build_new_image(
    logical_instance: &crate::ir::actor::LogicalActor,
    physical_instance: &mut crate::ir::actor::PhysicalActor,
    image_ident: crate::ir::behavior::BehaviorImageId,
    store: &crate::ir::support::image_cache::ImageCache,
) {
    let image_result = crate::ir::behavior::dialect::DialectRegistry::new_default().try_translate(&logical_instance.image.main_image, &image_ident);

    match image_result {
        Ok(image) => {
            physical_instance.image = actor::ImageState::Existing(image.clone());
            store.insert_blocking(image);
        }
        Err(e) => {
            log::warn!("Failed Compiling Image:\n{e}");
        }
    }
}
