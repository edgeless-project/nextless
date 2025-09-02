// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 University of Cambridge, System Research Group
// SPDX-FileCopyrightText: © 2024 Roman Kolcun <roman.kolcun@cl.cam.ac.uk>
// SPDX-License-Identifier: MIT

// https://rust-lang-nursery.github.io/rust-cookbook/compression/tar.html

mod cargo_messages;
pub(crate) mod cargo_target;

use super::BuildError;

#[allow(clippy::too_many_arguments)]
pub(crate) fn build_rust(
    cargo_project_path: std::path::PathBuf,
    enabled_features: Vec<String>,
    enable_default_features: bool,
    enable_all_features: bool,
    target_configuration: String,
    target_out_dir: String,
    target_prefix: String,
    target_suffix: String,
    build_std: Vec<String>,
    build_dir: std::path::PathBuf,
) -> Result<String, BuildError> {
    check_cargo(!build_std.is_empty())?;
    check_rustc(!build_std.is_empty())?;

    let build_dir_s = build_dir.to_str().ok_or_else(|| BuildError::Toolchain {
        msg: "Could not get build_dir str".to_string(),
        source: None,
    })?;

    let meta_out = std::process::Command::new("cargo")
        .current_dir(&cargo_project_path)
        .arg("metadata")
        .arg("--no-deps")
        .output()
        .map_err(|e| BuildError::Toolchain {
            msg: "Could not fetch cargo metdata".to_string(),
            source: Some(e.into()),
        })?;

    let meta: cargo_messages::MetadataRoot = serde_json::from_slice(&meta_out.stdout).map_err(|e| BuildError::Package {
        msg: "Could not parse cargo metdata.".to_string(),
        source: Some(e.into()),
    })?;

    // https://users.rust-lang.org/t/vector-first-with-length-check-before/80665/4
    let package = match &meta.packages[..] {
        [p] => p,
        _ => {
            return Err(BuildError::Package {
                msg: "Cargo.toml does not exactly one package.".to_string(),
                source: None,
            });
        }
    };

    let mut missing_features = Vec::new();
    for f in &enabled_features {
        if !package.features.contains_key(f) {
            missing_features.push(f.clone());
        }
    }

    if !missing_features.is_empty() {
        return Err(BuildError::Package {
            msg: format!("Crate is missing expected features: {}", missing_features.join(",")),
            source: None,
        });
    }

    let target = package
        .targets
        .iter()
        .find(|t| t.crate_types.contains(&"cdylib".to_string()))
        .ok_or_else(|| BuildError::Package {
            msg: "Cargo.toml does not contain a cdylib target".to_string(),
            source: None,
        })?;

    let lib_name = target.name.clone();

    let mut cmd = std::process::Command::new("cargo");
    cmd.current_dir(&cargo_project_path)
        .arg("build")
        .arg("--release")
        .arg(format!("--target={target_configuration}"))
        .arg("--message-format=json")
        .arg(format!("--target-dir={build_dir_s}"));

    if enable_all_features {
        cmd.arg("--all-features");
    }
    if !enable_default_features {
        cmd.arg("--no-default-features");
    }
    if !enabled_features.is_empty() {
        let features = enabled_features.join(",");
        cmd.arg(format!("--features={features}"));
    }

    if !build_std.is_empty() {
        let build_std_components = build_std.join(",");
        cmd.arg(format!("-Zbuild-std={build_std_components}"));
    }

    let build_output = cmd.output().map_err(|e| BuildError::Toolchain {
        msg: format!("Could not call cargo build: {cmd:?}."),
        source: Some(e.into()),
    })?;
    if !build_output.status.success() {
        let out_str = String::from_utf8(build_output.stdout).map_err(|e| BuildError::Toolchain {
            msg: "Could not parse build output".to_string(),
            source: Some(e.into()),
        })?;

        let mut compiler_errors = String::new();

        for line in out_str.lines() {
            if let Ok(cargo_messages::CargoBuildOutput::CompilerMessage { target, message }) =
                serde_json::from_str::<cargo_messages::CargoBuildOutput>(line)
            {
                compiler_errors += &format!("Crate {}:\n{}\n", target.name, message.rendered);
            }
        }

        if compiler_errors.len() == 0 {
            let err_str = String::from_utf8(build_output.stderr).map_err(|e| BuildError::Toolchain {
                msg: "Could not parse build stderr".to_string(),
                source: Some(e.into()),
            })?;

            compiler_errors = err_str;
        }

        return Err(BuildError::Compiler {
            msg: compiler_errors,
            source: None,
        });
    }

    let lib_file = build_dir.join(format!("{target_out_dir}/release/{target_prefix}{lib_name}{target_suffix}"));

    if lib_file.exists() {
        Ok(lib_file
            .to_str()
            .ok_or(BuildError::Toolchain {
                msg: "Path Error (raw library path)".to_string(),
                source: None,
            })?
            .to_string())
    } else {
        Err(BuildError::Toolchain {
            msg: "Library File does not exists".to_string(),
            source: None,
        })
    }
}

pub fn package_rust(function_source_dir: String) -> Result<String, BuildError> {
    check_cargo(true)?;

    let cargo_project_path = std::fs::canonicalize(function_source_dir.clone()).map_err(|e| BuildError::Package {
        msg: format!("Bad Path: {function_source_dir}."),
        source: Some(e.into()),
    })?;
    //
    let mut file_list_cmd = std::process::Command::new("cargo");
    file_list_cmd
        .current_dir(&cargo_project_path)
        .arg("package")
        .arg("--allow-dirty")
        .arg("--exclude-lockfile")
        .arg("--message-format=json")
        .arg("-Zunstable-options")
        .arg("-l");

    let file_list_output = file_list_cmd.output().map_err(|e| BuildError::Toolchain {
        msg: format!("Could not call cargo package -l: {file_list_cmd:?}."),
        source: Some(e.into()),
    })?;

    if !file_list_output.status.success() {
        return Err(BuildError::Toolchain {
            msg: "Cold not get list of crate files".to_string(),
            source: None,
        });
    }

    let file_list: cargo_messages::PackageListRoot = serde_json::from_slice(&file_list_output.stdout).map_err(|e| BuildError::Toolchain {
        msg: "Could not parse package list output".to_string(),
        source: Some(e.into()),
    })?;

    let crate_archive = std::env::temp_dir().join(format!("edgeless-tar-{}.tar.gz", uuid::Uuid::new_v4()));

    let tgz_file = std::fs::File::create(crate_archive.clone()).map_err(|e| BuildError::Toolchain {
        msg: "Could not create crate .tar.gz".to_string(),
        source: Some(e.into()),
    })?;

    let enc = flate2::write::GzEncoder::new(tgz_file, flate2::Compression::default());
    let mut tar = tar::Builder::new(enc);

    let sources: Vec<_> = file_list
        .files
        .iter()
        .filter_map(|(relative, absolute)| {
            if relative.as_str() == ".cargo_vcs_info.json" {
                return None;
            }
            if relative.as_str() == "Cargo.toml.orig" {
                return None;
            }
            if relative.as_str() == ".gitignore" {
                return None;
            }

            if let Some(absolute) = &absolute.path {
                return Some((relative, absolute));
            }

            None
        })
        .map(|(relative, absolute)| (absolute.clone(), relative.clone()))
        .collect();
    for (src_src, src_dest) in sources {
        tar.append_path_with_name(src_src, src_dest.clone()).map_err(|e| BuildError::Toolchain {
            msg: format!("Could not package file: {src_dest}"),
            source: Some(e.into()),
        })?;
    }

    if crate_archive.exists() {
        Ok(crate_archive
            .to_str()
            .ok_or(BuildError::Toolchain {
                msg: "Path Error (crate tar.gz)".to_string(),
                source: None,
            })?
            .to_string())
    } else {
        Err(BuildError::Toolchain {
            msg: "Crate .tar.gz does not exist".to_string(),
            source: None,
        })
    }
}

pub fn unpack_rust_package(rust_tar: &[u8]) -> Result<String, BuildError> {
    let dec = flate2::read::GzDecoder::new(rust_tar);
    let mut archive = tar::Archive::new(dec);
    let out_dir = std::env::temp_dir().join(format!("edgeless-source-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(out_dir.clone()).map_err(|e| BuildError::Toolchain {
        msg: "Could not create crate dir".to_string(),
        source: Some(e.into()),
    })?;
    archive.unpack(out_dir.clone()).map_err(|e| BuildError::Toolchain {
        msg: "Could not unpack crate".to_string(),
        source: Some(e.into()),
    })?;
    Ok(out_dir
        .to_str()
        .ok_or(BuildError::Toolchain {
            msg: "Path Error (crate dir)".to_string(),
            source: None,
        })?
        .to_string())
}

pub fn check_cargo(requires_nightly: bool) -> Result<(), crate::BuildError> {
    check_rust_component("cargo", None, requires_nightly)
}

pub fn check_rustc(requires_nightly: bool) -> Result<(), crate::BuildError> {
    check_rust_component("rustc", Some("rustc".to_string()), requires_nightly)
}

fn check_rust_component(name: &str, env_var: Option<String>, requires_nightly: bool) -> Result<(), crate::BuildError> {
    let binary = if let Some(env_var) = env_var {
        std::env::var(env_var).unwrap_or(name.to_string())
    } else {
        name.to_string()
    };

    let cargo_version_output = std::process::Command::new(binary)
        .arg("-V")
        .output()
        .map_err(|e| crate::BuildError::Toolchain {
            msg: format!("Could not fetch {name} version"),
            source: Some(e.into()),
        })?;

    if !cargo_version_output.status.success() {
        return Err(BuildError::Toolchain {
            msg: format!("{name} not found /failed"),
            source: None,
        });
    }

    if requires_nightly {
        let version_str = String::from_utf8(cargo_version_output.stdout).map_err(|e| BuildError::Toolchain {
            msg: format!("Could not parse {name} version"),
            source: Some(e.into()),
        })?;

        if !version_str.contains("nightly") {
            return Err(BuildError::Toolchain {
                msg: format!("System required nightly version of {name}. Got Version {version_str}"),
                source: None,
            });
        }
    }

    Ok(())
}
