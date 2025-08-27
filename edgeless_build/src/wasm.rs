// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 University of Cambridge, System Research Group
// SPDX-FileCopyrightText: © 2024 Roman Kolcun <roman.kolcun@cl.cam.ac.uk>
// SPDX-License-Identifier: MIT

use super::BuildError;

pub fn rust_to_wasm(
    function_source_dir: String,
    enabled_features: Vec<String>,
    enable_default_features: bool,
    enable_all_features: bool,
) -> Result<String, BuildError> {
    let cargo_project_path = std::fs::canonicalize(std::path::PathBuf::from(function_source_dir.clone())).map_err(|e| BuildError::Package {
        msg: format!("Bad Path: {function_source_dir}."),
        source: Some(e.into()),
    })?;

    let build_dir = std::env::temp_dir().join(format!("edgeless-{}", uuid::Uuid::new_v4()));

    let raw_result = crate::rust::build_rust(
        cargo_project_path.clone(),
        enabled_features,
        enable_default_features,
        enable_all_features,
        "wasm32-unknown-unknown".to_string(),
        "wasm32-unknown-unknown".to_string(),
        "".to_string(),
        ".wasm".to_string(),
        vec![],
        build_dir.clone(),
    )?;

    let out_file = build_dir
        .join("function.wasm")
        .to_str()
        .ok_or(BuildError::Toolchain {
            msg: "Path Error (output .wasm file)".to_string(),
            source: None,
        })?
        .to_string();

    let wasm_opt_res = std::process::Command::new("wasm-opt")
        .current_dir(&cargo_project_path)
        .arg("-Oz")
        .arg("--all-features")
        .arg("--enable-bulk-memory")
        .arg("-o")
        .arg(&out_file)
        .arg(&raw_result)
        .output()
        .map_err(|e| BuildError::Toolchain {
            msg: "Could not call wasm-opt.".to_string(),
            source: Some(e.into()),
        })?;

    if wasm_opt_res.status.success() {
        Ok(out_file)
    } else {
        Err(BuildError::Compiler {
            msg: format!(
                "Wasm Opt Failed:\nStdout: {}\nStderr: {}",
                String::from_utf8(wasm_opt_res.stdout).unwrap_or("STDOUT Not Parseable".to_string()),
                String::from_utf8(wasm_opt_res.stderr).unwrap_or("STDERR Not Parseable".to_string())
            ),
            source: None,
        })
    }
}
