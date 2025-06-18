// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

/// Struct representing the updatable callbacks/aliases of a function instance.
/// Shared between a function instance's host and guest.
#[derive(Clone)]
pub struct AliasMapping {
    input_mapping: std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<String, edgeless_api::common::Input>>>,
    output_mapping: std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<String, edgeless_api::common::Output>>>,
}

impl Default for AliasMapping {
    fn default() -> Self {
        Self::new()
    }
}

impl AliasMapping {
    pub fn new() -> Self {
        AliasMapping {
            input_mapping: std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
            output_mapping: std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
        }
    }

    pub async fn get_mapping(&self, alias: &str) -> Option<edgeless_api::common::Output> {
        self.output_mapping.lock().await.get(alias).cloned()
    }

    pub async fn update(
        &mut self,
        new_input_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, edgeless_api::common::Input>,
        new_output_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, edgeless_api::common::Output>,
    ) -> (
        (
            std::collections::HashMap<String, edgeless_api::common::Input>,
            std::collections::HashMap<String, edgeless_api::common::Output>,
        ),
        (
            std::collections::HashMap<String, edgeless_api::common::Input>,
            std::collections::HashMap<String, edgeless_api::common::Output>,
        ),
    ) {
        let mut new_input_mapping = new_input_mapping;
        let mut new_output_mapping = new_output_mapping;

        let mut removed_inputs = std::collections::HashMap::<String, edgeless_api::common::Input>::new();
        let mut removed_output = std::collections::HashMap::<String, edgeless_api::common::Output>::new();

        let mut added_inputs = std::collections::HashMap::<String, edgeless_api::common::Input>::new();
        let mut added_outputs = std::collections::HashMap::<String, edgeless_api::common::Output>::new();

        let mut lcked_inputs = self.input_mapping.lock().await;
        let mut lcked_outputs = self.output_mapping.lock().await;

        lcked_inputs.retain(|i_id, i| {
            match new_input_mapping.entry(edgeless_api::function_instance::PortId(i_id.to_string())) {
                std::collections::hash_map::Entry::Occupied(val) => {
                    if i != val.get() {
                        removed_inputs.insert(i_id.to_string(), i.clone());
                    }
                }
                std::collections::hash_map::Entry::Vacant(_) => {
                    return false;
                }
            }
            true
        });

        lcked_outputs.retain(|o_id, o| {
            match new_output_mapping.entry(edgeless_api::function_instance::PortId(o_id.to_string())) {
                std::collections::hash_map::Entry::Occupied(val) => {
                    if o != val.get() {
                        removed_output.insert(o_id.to_string(), o.clone());
                    }
                }
                std::collections::hash_map::Entry::Vacant(_) => {
                    return false;
                }
            }
            true
        });

        for (i_id, i) in new_input_mapping {
            match lcked_inputs.entry(i_id.0.clone()) {
                std::collections::hash_map::Entry::Occupied(mut val) => {
                    if val.get() != &i {
                        added_inputs.insert(i_id.0.clone(), i.clone());
                    }
                    val.insert(i);
                }
                std::collections::hash_map::Entry::Vacant(val) => {
                    val.insert(i.clone());
                    added_inputs.insert(i_id.0.clone(), i.clone());
                }
            }
        }

        for (o_id, o) in new_output_mapping {
            match lcked_outputs.entry(o_id.0.clone()) {
                std::collections::hash_map::Entry::Occupied(mut val) => {
                    if val.get() != &o {
                        added_outputs.insert(o_id.0.clone(), o.clone());
                    }
                    val.insert(o);
                }
                std::collections::hash_map::Entry::Vacant(val) => {
                    val.insert(o.clone());
                    added_outputs.insert(o_id.0.clone(), o.clone());
                }
            }
        }

        ((removed_inputs, removed_output), (added_inputs, added_outputs))
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    #[tokio::test]
    async fn updated_output_link() {
        let a_port = edgeless_api::function_instance::PortId("a".to_string());
        let b_port = edgeless_api::function_instance::PortId("b".to_string());
        let initial_output =
            edgeless_api::common::Output::Single(edgeless_api::function_instance::InstanceId::new(uuid::Uuid::new_v4()), b_port.clone());
        let link_output = edgeless_api::common::Output::Link(edgeless_api::link::LinkInstanceId(uuid::Uuid::new_v4()));

        let mut a = AliasMapping::new();
        let ((removed_inputs, removed_outputs), (added_inputs, mut added_outputs)) = a
            .update(
                std::collections::HashMap::new(),
                std::collections::HashMap::from([(a_port.clone(), initial_output.clone())]),
            )
            .await;

        assert_eq!(removed_inputs.len(), 0);
        assert_eq!(added_inputs.len(), 0);
        assert_eq!(added_outputs.len(), 1);
        assert_eq!(removed_outputs.len(), 0);
        assert_eq!(added_outputs.remove("a").unwrap(), initial_output.clone());

        let ((removed_inputs2, mut removed_outputs2), (added_inputs2, mut added_outputs2)) = a
            .update(
                std::collections::HashMap::new(),
                std::collections::HashMap::from([(a_port.clone(), link_output.clone())]),
            )
            .await;

        assert_eq!(removed_inputs2.len(), 0);
        assert_eq!(added_inputs2.len(), 0);
        assert_eq!(added_outputs2.len(), 1);
        assert_eq!(added_outputs2.remove("a").unwrap(), link_output.clone());
        assert_eq!(removed_outputs2.len(), 1);
        assert_eq!(removed_outputs2.remove("a").unwrap(), initial_output.clone());
    }
}
