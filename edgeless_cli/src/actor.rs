// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

#[derive(Debug, clap::Subcommand)]
pub(crate) enum FunctionCommands {
    Build { spec_file: String },
    Package { spec_file: String },
}

#[derive(thiserror::Error, Debug)]
pub enum ActorError {
    #[error("Error with provided input file '{file}'")]
    InputError { file: String, source: anyhow::Error },
    #[error("Error creating output file '{file}'")]
    OutputError { file: String, source: anyhow::Error },
    #[error("Could not load actor description.")]
    StarlarkConfigurationError(#[from] edgeless_config::ConfigError),
    #[error("Configuration's entrypoint is not an actor description")]
    NotAnActor,
    #[error("Could not complete build command")]
    BuildError(#[from] edgeless_build::BuildError),
}

pub(crate) async fn actor_command(command: FunctionCommands, _config: crate::CLiConfig) -> Result<(), ActorError> {
    match command {
        FunctionCommands::Build { spec_file } => build_actor(spec_file).await?,
        FunctionCommands::Package { spec_file } => package_actor(spec_file).await?,
    }
    Ok(())
}

pub(crate) async fn build_actor(spec_file: String) -> Result<(), ActorError> {
    let (actor_spec, cargo_project_path) = load_actor_specification(spec_file)?;

    write_legacy_actor_specification(&actor_spec, &cargo_project_path)?;

    let out_file = cargo_project_path.join(format!("{}.wasm", actor_spec.id));

    let mut port_features: Vec<_> = actor_spec.inputs.keys().map(|i| format!("input_{i}")).collect();
    port_features.append(&mut actor_spec.outputs.keys().map(|o| format!("output_{o}")).collect());

    let build_result_path = edgeless_build::wasm::rust_to_wasm(&cargo_project_path, port_features, true, false)?;

    std::fs::copy(&build_result_path, &out_file).map_err(|e| ActorError::OutputError {
        file: out_file.to_string_lossy().to_string(),
        source: e.into(),
    })?;

    Ok(())
}

pub(crate) async fn package_actor(spec_file: String) -> Result<(), ActorError> {
    let (actor_spec, cargo_project_path) = load_actor_specification(spec_file)?;

    write_legacy_actor_specification(&actor_spec, &cargo_project_path)?;

    let out_file_path = cargo_project_path.join(format!("{}.tar.gz", actor_spec.id));

    let packaged = edgeless_build::rust::package_rust(cargo_project_path)?;
    std::fs::copy(&packaged, &out_file_path).map_err(|e| ActorError::OutputError {
        file: out_file_path.to_string_lossy().to_string(),
        source: e.into(),
    })?;
    Ok(())
}

fn load_actor_specification(spec_file_path: String) -> Result<(edgeless_config::actor_class::EdgelessActorClass, std::path::PathBuf), ActorError> {
    let canonical_spec_file_path = std::fs::canonicalize(std::path::PathBuf::from(spec_file_path.clone())).map_err(|e| ActorError::InputError {
        file: spec_file_path.clone(),
        source: anyhow::Error::from(e).context("Cound not canonicalize path"),
    })?;

    let cargo_project_path = canonical_spec_file_path
        .parent()
        .ok_or(ActorError::InputError {
            file: spec_file_path.clone(),
            source: anyhow::anyhow!("Could not get parent directory"),
        })?
        .to_path_buf();

    let actor_spec_file_extension = canonical_spec_file_path.extension().ok_or(ActorError::InputError {
        file: canonical_spec_file_path.to_string_lossy().to_string(),
        source: anyhow::anyhow!("Could not determine file extension"),
    })?;

    if actor_spec_file_extension == "json" {
        let config_json = std::fs::read_to_string(&canonical_spec_file_path).map_err(|e| ActorError::InputError {
            file: canonical_spec_file_path.to_string_lossy().to_string(),
            source: e.into(),
        })?;

        let actor_config = serde_json::from_str(&config_json).map_err(|e| ActorError::InputError {
            file: canonical_spec_file_path.to_string_lossy().to_string(),
            source: e.into(),
        })?;

        Ok((actor_config, cargo_project_path))
    } else {
        match edgeless_config::load(canonical_spec_file_path)? {
            edgeless_config::LoadResult::ActorClass(actor_spec) => Ok((actor_spec, cargo_project_path)),
            _ => {
                panic!("Can't Spawn Function as Workflow");
            }
        }
    }
}

// The build macros still rely on a  'function.json'.
fn write_legacy_actor_specification(
    actor_spec: &edgeless_config::actor_class::EdgelessActorClass,
    project_dir_path: &std::path::Path,
) -> Result<(), ActorError> {
    let out_file_path = project_dir_path.join("function.json");

    let spec_json = serde_json::to_vec(&actor_spec).map_err(|e| ActorError::OutputError {
        file: out_file_path.to_string_lossy().to_string(),
        source: e.into(),
    })?;

    std::fs::write(out_file_path.clone(), spec_json).map_err(|e| ActorError::OutputError {
        file: out_file_path.to_string_lossy().to_string(),
        source: e.into(),
    })
}
