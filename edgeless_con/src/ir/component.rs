// SPDX-FileCopyrightText: © 2026 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ScalingMode {
    Singleton,
    Scalable { min_instances: usize, max_instances: usize },
    AllNodes,
}

#[derive(Clone, Default, Debug)]
pub struct NodeFilters {
    pub node_ids_allowed: Option<Vec<edgeless_api::function_instance::NodeId>>,
    pub node_ids_denied: Option<Vec<edgeless_api::function_instance::NodeId>>,
    pub runtime_dialects_allowed: Option<std::collections::HashSet<super::behavior::dialect::DialectId>>,
    pub runtime_dialects_denied: Option<std::collections::HashSet<super::behavior::dialect::DialectId>>,
    pub node_label_filter_allowed: Option<Vec<std::collections::HashSet<String>>>,
    pub node_label_filter_denied: Option<Vec<std::collections::HashSet<String>>>,
    pub cluster_ids_allowed: Option<Vec<edgeless_api::function_instance::NodeId>>,
    pub cluster_ids_denied: Option<Vec<edgeless_api::function_instance::NodeId>>,
    pub node_id_init_on: Option<edgeless_api::function_instance::NodeId>,
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

impl NodeFilters {
    /// Deployment requirements from the annotations in the function's spawn request.
    pub fn from_annotations(annotations: &std::collections::HashMap<String, String>) -> Self {
        let mut node_ids_allowed = None;
        if let Some(val) = annotations.get("node_ids_allowed") {
            node_ids_allowed = Some(val.split(",").filter_map(|x| uuid::Uuid::parse_str(x).ok()).collect());
        }

        let mut node_id_init_on = None;
        if let Some(val) = annotations.get("node_id_init_on") {
            node_id_init_on = uuid::Uuid::parse_str(val).ok();
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
            node_id_init_on,
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

        let parsed = crate::ir::component::ScalingMode::from_annotations(&annotations);

        assert_eq!(parsed, crate::ir::component::ScalingMode::AllNodes);
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
            crate::ir::component::ScalingMode::Scalable {
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

        assert_eq!(parsed, crate::ir::component::ScalingMode::Singleton);
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
            ("node_id_init_on", "00000000-0000-0000-0000-000000000009"),
        ]);

        let annotations = annotations
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect::<std::collections::HashMap<String, String>>();

        let parsed = super::NodeFilters::from_annotations(&annotations);

        assert_eq!(
            parsed.node_ids_allowed.unwrap(),
            vec![
                uuid::Uuid::from_str("00000000-0000-0000-0000-000000000001").unwrap(),
                uuid::Uuid::from_str("00000000-0000-0000-0000-000000000002").unwrap()
            ]
        );

        assert_eq!(
            parsed.node_id_init_on.unwrap(),
            uuid::Uuid::from_str("00000000-0000-0000-0000-000000000009").unwrap(),
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
