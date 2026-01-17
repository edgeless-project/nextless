// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

#[derive(Clone)]
pub struct LogicalActor {
    pub image: super::behavior::Behavior,
    pub annotations: std::collections::HashMap<String, String>,
    pub scaling_mode: ScalingMode,
    pub node_filter: NodeFilter,
    pub logical_ports: super::LogicalPorts,
    pub instances: Vec<std::cell::RefCell<super::PhysicalComponentState>>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ScalingMode {
    Singleton,
    Scalable { min_instances: usize, max_instances: usize },
    AllNodes,
}

#[derive(Clone, Default)]
pub struct NodeFilter {
    pub node_ids_allowed: Option<Vec<edgeless_api::function_instance::NodeId>>,
    pub node_ids_denied: Option<Vec<edgeless_api::function_instance::NodeId>>,
    pub runtime_dialects_allowed: Option<std::collections::HashSet<super::behavior::dialect::DialectId>>,
    pub runtime_dialects_denied: Option<std::collections::HashSet<super::behavior::dialect::DialectId>>,
    pub node_label_filter_allowed: Option<Vec<std::collections::HashSet<String>>>,
    pub node_label_filter_denied: Option<Vec<std::collections::HashSet<String>>>,
    pub cluster_ids_allowed: Option<Vec<edgeless_api::function_instance::NodeId>>,
    pub cluster_ids_denied: Option<Vec<edgeless_api::function_instance::NodeId>>,
}

#[derive(Clone)]
pub struct PhysicalActor {
    pub(crate) id: edgeless_api::function_instance::InstanceId,
    pub(crate) component_name: String,
    pub(crate) creation_time: std::time::Instant,
    pub(crate) image: ImageState,
    // Temporary is the Function API still is based on older types
    pub(crate) behavior_spec: super::behavior::BehaviorSpec,
    pub(crate) desired_mapping: super::PhysicalPorts,
    pub(crate) materialized: Option<std::cell::RefCell<MaterializedActor>>,
    pub(crate) annotations: std::collections::HashMap<String, String>,
}

#[derive(Clone)]
pub enum ImageState {
    Planned(super::behavior::BehaviorImageId),
    Existing(super::behavior::BehaviorImage),
}

impl super::LogicalComponent for LogicalActor {
    fn logical_ports(&self) -> &super::LogicalPorts {
        &self.logical_ports
    }

    fn logical_ports_mut(&mut self) -> &mut super::LogicalPorts {
        &mut self.logical_ports
    }

    fn instance_ids(&mut self) -> Vec<edgeless_api::function_instance::InstanceId> {
        self.instances.iter().filter_map(|i| i.borrow().id()).collect()
    }

    fn split_view(&mut self) -> (&mut super::LogicalPorts, Vec<&std::cell::RefCell<super::PhysicalComponentState>>) {
        (&mut self.logical_ports, self.instances.iter().collect())
    }

    fn instances(&self) -> Vec<&std::cell::RefCell<super::PhysicalComponentState>> {
        self.instances.iter().collect()
    }
}

impl super::PhysicalComponent for PhysicalActor {
    fn physical_ports(&mut self) -> &mut super::PhysicalPorts {
        &mut self.desired_mapping
    }

    fn materialized_state(&self) -> Option<&std::cell::RefCell<dyn super::MaterializedComponent>> {
        self.materialized
            .as_ref()
            .map(|v| v as &std::cell::RefCell<dyn super::MaterializedComponent>)
    }

    fn id(&self) -> edgeless_api::function_instance::InstanceId {
        self.id
    }

    fn creation_time(&self) -> std::time::Instant {
        self.creation_time
    }

    fn materialize(&mut self, telemetry_provider: &Option<Box<dyn super::TelemetryProvider>>) -> Vec<super::RequiredChange> {
        let mut changes = Vec::new();
        if let Some(materialized) = &self.materialized {
            let mut materialized = materialized.borrow_mut();
            if !materialized.mapping.is_current_mapping(&self.desired_mapping) {
                changes.push(super::RequiredChange::PatchFunction {
                    function_id: self.id,
                    function_name: self.component_name.clone(),
                    input_mapping: self.desired_mapping.physical_input_mapping.clone(),
                    output_mapping: self.desired_mapping.physical_output_mapping.clone(),
                });
                materialized.mapping.update(&self.desired_mapping, &self.id, telemetry_provider);
            }
        } else {
            changes.push(super::RequiredChange::StartFunction {
                function_id: self.id,
                function_name: self.component_name.clone(),
                image: match &self.image {
                    ImageState::Planned(_) => {
                        // Need proper handling for this
                        panic!("Image Creation Failed");
                    }
                    ImageState::Existing(behavior_image) => behavior_image.clone(),
                },
                behavior_spec: self.behavior_spec.clone(),
                input_mapping: self.desired_mapping.physical_input_mapping.clone(),
                output_mapping: self.desired_mapping.physical_output_mapping.clone(),
                annotations: self.annotations.clone(),
            });

            let mut ports = super::MaterializedPorts::default();
            ports.update(&self.desired_mapping, &self.id, telemetry_provider);

            self.materialized = Some(std::cell::RefCell::new(super::actor::MaterializedActor {
                mapping: ports,
                runtime_statistics: telemetry_provider.as_ref().map(|t| t.component_statistics_for(&self.id)),
            }))
        }
        changes
    }

    fn as_actor(&self) -> Option<&self::PhysicalActor> {
        Some(self)
    }

    fn as_actor_mut(&mut self) -> Option<&mut self::PhysicalActor> {
        Some(self)
    }

    fn stop(&mut self) -> Vec<super::RequiredChange> {
        vec![super::RequiredChange::StopFunction { function_id: self.id }]
    }
}

#[derive(Clone)]
pub struct MaterializedActor {
    pub(crate) mapping: super::MaterializedPorts,
    pub(crate) runtime_statistics: Option<Box<dyn super::ComponentRuntimeStatistics>>,
}

impl super::MaterializedComponent for MaterializedActor {
    fn materialized_ports(&mut self) -> &mut super::MaterializedPorts {
        &mut self.mapping
    }

    fn runtime_statistics(&self) -> Option<&dyn super::ComponentRuntimeStatistics> {
        self.runtime_statistics.as_deref()
    }
}

impl LogicalActor {
    pub(crate) fn enabled_inputs(&self) -> Vec<edgeless_api::function_instance::PortId> {
        self.logical_ports.logical_input_mapping.iter().map(|i| i.0.clone()).collect()
    }

    pub(crate) fn enabled_outputs(&self) -> Vec<edgeless_api::function_instance::PortId> {
        self.logical_ports.logical_output_mapping.iter().map(|i| i.0.clone()).collect()
    }
}

impl From<edgeless_api::workflow_instance::WorkflowFunction> for LogicalActor {
    fn from(function_req: edgeless_api::workflow_instance::WorkflowFunction) -> Self {
        let scaling_mode = ScalingMode::from_annotations(&function_req.annotations);
        let node_filters = NodeFilter::from_annotations(&function_req.annotations);
        //
        Self {
            image: super::behavior::Behavior::try_from(function_req.behavior).unwrap(),
            instances: Vec::new(),

            annotations: function_req.annotations,
            logical_ports: super::LogicalPorts {
                logical_input_mapping: super::logical_model::parse_api_input_mapping(function_req.input_mapping),
                logical_output_mapping: super::logical_model::parse_api_output_mapping(function_req.output_mapping),
            },
            scaling_mode,
            node_filter: node_filters,
        }
    }
}

impl ScalingMode {
    pub fn from_annotations(annotations: &std::collections::HashMap<String, String>) -> Self {
        let min_instances = annotations
            .get("min_instances")
            .map(|val| val.as_str())
            .unwrap_or("1")
            .parse::<usize>()
            .unwrap_or(1);
        let max_instances = annotations
            .get("max_instances")
            .map(|val| val.as_str())
            .unwrap_or("99")
            .parse::<usize>()
            .unwrap_or(99);

        match annotations.get("scaling_mode").map(|mode| mode.as_str()).unwrap_or("singleton") {
            "all_nodes" => ScalingMode::AllNodes,
            "scalable" => ScalingMode::Scalable {
                min_instances,
                max_instances,
            },
            _ => ScalingMode::Singleton,
        }
    }
}

impl NodeFilter {
    /// Deployment requirements from the annotations in the function's spawn request.
    pub fn from_annotations(annotations: &std::collections::HashMap<String, String>) -> Self {
        let mut node_ids_allowed = None;
        if let Some(val) = annotations.get("node_ids_allowed") {
            node_ids_allowed = Some(val.split(",").filter_map(|x| uuid::Uuid::parse_str(x).ok()).collect());
        }

        let mut node_ids_denied = None;
        if let Some(val) = annotations.get("node_ids_denied") {
            node_ids_denied = Some(val.split(",").filter_map(|x| uuid::Uuid::parse_str(x).ok()).collect());
        }

        let mut runtime_dialects_allowed = None;
        if let Some(val) = annotations.get("runtime_dialects_allowed") {
            runtime_dialects_allowed = Some(
                val.split(",")
                    .filter_map(|x| parse_dialect_id(x))
                    .collect::<std::collections::HashSet<_>>(),
            );
        }

        let mut runtime_dialects_denied = None;
        if let Some(val) = annotations.get("runtime_dialects_denied") {
            runtime_dialects_denied = Some(
                val.split(",")
                    .filter_map(|x| parse_dialect_id(x))
                    .collect::<std::collections::HashSet<_>>(),
            );
        }

        let mut node_label_filter_allowed = None;
        if let Some(val) = annotations.get("node_label_filter_allowed") {
            let alternatives = val.split("|");

            let mut parsed_alternatives = Vec::new();

            for alternative in alternatives {
                let combined: std::collections::HashSet<_> = alternative.split("&").map(String::from).collect();
                parsed_alternatives.push(combined);
            }
            node_label_filter_allowed = Some(parsed_alternatives);
        }

        let mut node_label_filter_denied = None;
        if let Some(val) = annotations.get("node_label_filter_denied") {
            let alternatives = val.split("|");

            let mut parsed_alternatives = Vec::new();

            for alternative in alternatives {
                let combined: std::collections::HashSet<_> = alternative.split("&").map(String::from).collect();
                parsed_alternatives.push(combined);
            }
            node_label_filter_denied = Some(parsed_alternatives);
        }

        let mut cluster_ids_allowed = None;
        if let Some(val) = annotations.get("cluster_ids_allowed") {
            cluster_ids_allowed = Some(val.split(",").filter_map(|x| uuid::Uuid::parse_str(x).ok()).collect());
        }

        let mut cluster_ids_denied = None;
        if let Some(val) = annotations.get("cluster_ids_denied") {
            cluster_ids_denied = Some(val.split(",").filter_map(|x| uuid::Uuid::parse_str(x).ok()).collect());
        }

        Self {
            node_ids_allowed,
            node_ids_denied,
            runtime_dialects_allowed,
            runtime_dialects_denied,
            node_label_filter_allowed,
            node_label_filter_denied,
            cluster_ids_allowed,
            cluster_ids_denied,
        }
    }
}

fn parse_dialect_id(dialect_string: &str) -> Option<crate::ir::behavior::dialect::DialectId> {
    match dialect_string {
        "NATIVE_DYNAMIC" => Some(super::behavior::dialect::native_dyanamic::ID),
        "RUST" => Some(super::behavior::dialect::rust::ID),
        "WASM" => Some(super::behavior::dialect::wasm::ID),
        _ => None,
    }
}

#[cfg(test)]
mod parser_test {
    use std::str::FromStr;

    #[test]
    fn scaling_mode_valid_all_nodes() {
        let annotations = std::collections::HashMap::from([("scaling_mode", "all_nodes"), ("min_instances", "10"), ("max_instances", "25")]);

        let annotations = annotations
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect::<std::collections::HashMap<String, String>>();

        let parsed = super::ScalingMode::from_annotations(&annotations);

        assert_eq!(parsed, crate::ir::actor::ScalingMode::AllNodes);
    }

    #[test]
    fn scaling_mode_valid_scalable() {
        let annotations = std::collections::HashMap::from([("scaling_mode", "scalable"), ("min_instances", "10"), ("max_instances", "25")]);

        let annotations = annotations
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect::<std::collections::HashMap<String, String>>();

        let parsed = super::ScalingMode::from_annotations(&annotations);

        assert_eq!(
            parsed,
            crate::ir::actor::ScalingMode::Scalable {
                min_instances: 10,
                max_instances: 25
            }
        );
    }

    #[test]
    fn scaling_mode_valid_singleton() {
        let annotations = std::collections::HashMap::from([("scaling_mode", "singleton"), ("min_instances", "10"), ("max_instances", "25")]);

        let annotations = annotations
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect::<std::collections::HashMap<String, String>>();

        let parsed = super::ScalingMode::from_annotations(&annotations);

        assert_eq!(parsed, crate::ir::actor::ScalingMode::Singleton);
    }

    #[test]
    fn node_filter_parse_valid() {
        let annotations = std::collections::HashMap::from([
            (
                "node_ids_allowed",
                "00000000-0000-0000-0000-000000000001,00000000-0000-0000-0000-000000000002",
            ),
            (
                "node_ids_denied",
                "00000000-0000-0000-0000-000000000003,00000000-0000-0000-0000-000000000004",
            ),
            (
                "cluster_ids_allowed",
                "00000000-0000-0000-0000-000000000005,00000000-0000-0000-0000-000000000006",
            ),
            (
                "cluster_ids_denied",
                "00000000-0000-0000-0000-000000000007,00000000-0000-0000-0000-000000000008",
            ),
            ("runtime_dialects_allowed", "WASM,NATIVE_DYNAMIC"),
            ("runtime_dialects_denied", "RUST"),
            ("node_label_filter_allowed", "a&b&c|d&e|f"),
            ("node_label_filter_denied", "g&h&i|j&k|l"),
        ]);

        let annotations = annotations
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect::<std::collections::HashMap<String, String>>();

        let parsed = super::NodeFilter::from_annotations(&annotations);

        assert_eq!(
            parsed.node_ids_allowed.unwrap(),
            vec![
                uuid::Uuid::from_str("00000000-0000-0000-0000-000000000001").unwrap(),
                uuid::Uuid::from_str("00000000-0000-0000-0000-000000000002").unwrap()
            ]
        );

        assert_eq!(
            parsed.node_ids_denied.unwrap(),
            vec![
                uuid::Uuid::from_str("00000000-0000-0000-0000-000000000003").unwrap(),
                uuid::Uuid::from_str("00000000-0000-0000-0000-000000000004").unwrap()
            ]
        );

        assert_eq!(
            parsed.cluster_ids_allowed.unwrap(),
            vec![
                uuid::Uuid::from_str("00000000-0000-0000-0000-000000000005").unwrap(),
                uuid::Uuid::from_str("00000000-0000-0000-0000-000000000006").unwrap()
            ]
        );

        assert_eq!(
            parsed.cluster_ids_denied.unwrap(),
            vec![
                uuid::Uuid::from_str("00000000-0000-0000-0000-000000000007").unwrap(),
                uuid::Uuid::from_str("00000000-0000-0000-0000-000000000008").unwrap()
            ]
        );

        assert_eq!(
            parsed.runtime_dialects_allowed.unwrap(),
            std::collections::HashSet::from([crate::ir::behavior::dialect::wasm::ID, crate::ir::behavior::dialect::native_dyanamic::ID])
        );

        assert_eq!(
            parsed.runtime_dialects_denied.unwrap(),
            std::collections::HashSet::from([crate::ir::behavior::dialect::rust::ID])
        );

        assert_eq!(
            parsed.node_label_filter_allowed.unwrap(),
            vec![
                std::collections::HashSet::from(["a".to_string(), "b".to_string(), "c".to_string()]),
                std::collections::HashSet::from(["d".to_string(), "e".to_string()]),
                std::collections::HashSet::from(["f".to_string()])
            ]
        );

        assert_eq!(
            parsed.node_label_filter_denied.unwrap(),
            vec![
                std::collections::HashSet::from(["g".to_string(), "h".to_string(), "i".to_string()]),
                std::collections::HashSet::from(["j".to_string(), "k".to_string()]),
                std::collections::HashSet::from(["l".to_string()])
            ]
        );
    }
}
