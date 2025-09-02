// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 University of Cambridge, System Research Group
// SPDX-FileCopyrightText: © 2024 Roman Kolcun <roman.kolcun@cl.cam.ac.uk>
// SPDX-License-Identifier: MIT

// We use a custom bare metal target for native (aarch64-edgeless-none-actor.json):
// https://lowenware.com/blog/aarch64-bare-metal-program-in-rust/

use super::BuildError;

#[derive(PartialEq, Eq)]
pub enum NativeTarget {
    AARCH64,
    AMD64,
}

#[derive(PartialEq, Eq)]
pub enum NativeFeature {
    Aes,
}

pub fn rust_to_dynlib(
    function_source_dir: String,
    enabled_features: Vec<String>,
    enable_default_features: bool,
    enable_all_features: bool,
    target: NativeTarget,
    features: Vec<NativeFeature>,
) -> Result<String, BuildError> {
    let cargo_project_path = std::fs::canonicalize(std::path::PathBuf::from(function_source_dir.clone())).map_err(|e| BuildError::Package {
        msg: format!("Bad Path: {function_source_dir}."),
        source: Some(e.into()),
    })?;

    let build_dir = std::env::temp_dir().join(format!("edgeless-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&build_dir).map_err(|e| BuildError::Toolchain {
        msg: "Could not write target config".to_string(),
        source: Some(e.into()),
    })?;

    let (target_tripple, target_desc) = match target {
        NativeTarget::AARCH64 => {
            let extra_features = if features.contains(&NativeFeature::Aes) {
                vec!["+aes".to_string()]
            } else {
                Vec::new()
            };

            let target_desc = super::rust::cargo_target::aarch64_target(extra_features);

            ("aarch64-edgeless-none-actor", target_desc)
        }
        NativeTarget::AMD64 => {
            let extra_features = Vec::new();
            let target_desc = super::rust::cargo_target::amd64_target(extra_features);

            ("x86_64-edgeless-none-actor", target_desc)
        }
    };

    let config_str = serde_json::to_string(&target_desc).map_err(|e| BuildError::Toolchain {
        msg: "Could not serialize target config".to_string(),
        source: Some(e.into()),
    })?;
    let config_path = build_dir.join(format!("{target_tripple}.json"));
    std::fs::write(&config_path, config_str).map_err(|e| BuildError::Toolchain {
        msg: "Could not write target config".to_string(),
        source: Some(e.into()),
    })?;

    let config_path_str = config_path
        .to_str()
        .ok_or(BuildError::Toolchain {
            msg: "Target config file path string error".to_string(),
            source: None,
        })?
        .to_string();

    crate::rust::build_rust(
        cargo_project_path.clone(),
        enabled_features,
        enable_default_features,
        enable_all_features,
        config_path_str,
        target_tripple.to_string(),
        "".to_string(),
        ".actor".to_string(),
        vec!["compiler_builtins".to_string(), "core".to_string(), "alloc".to_string()],
        build_dir.clone(),
    )
}
