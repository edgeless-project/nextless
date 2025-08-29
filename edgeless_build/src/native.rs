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

    let (target_tripple, target_desc) = match target {
        NativeTarget::AARCH64 => {
            let build_config = if features.contains(&NativeFeature::Aes) {
                format!("{}/build_config/aarch64/aarch64-edgeless-none-actor.json", env!("CARGO_MANIFEST_DIR"))
            } else {
                format!("{}/build_config/aarch64_aes/aarch64-edgeless-none-actor.json", env!("CARGO_MANIFEST_DIR"))
            };
            ("aarch64-edgeless-none-actor", build_config)
        }
        NativeTarget::AMD64 => {
            // "amd64-edgeless-none-actor",
            return Err(BuildError::Toolchain {
                msg: "Native AMD64 Currently Unimplemented".to_string(),
                source: None,
            });
        }
    };

    crate::rust::build_rust(
        cargo_project_path.clone(),
        enabled_features,
        enable_default_features,
        enable_all_features,
        target_desc,
        target_tripple.to_string(),
        "".to_string(),
        ".actor".to_string(),
        vec!["compiler_builtins".to_string(), "core".to_string(), "alloc".to_string()],
        build_dir.clone(),
    )
}
