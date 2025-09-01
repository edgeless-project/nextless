// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

#[derive(Debug, clap::Subcommand)]
pub(crate) enum FunctionCommands {
    Build { spec_file: String },
    Package { spec_file: String },
}

pub(crate) async fn actor_command(command: FunctionCommands, _config: crate::CLiConfig) -> Result<(), anyhow::Error> {
    match command {
        FunctionCommands::Build { spec_file } => build_actor(spec_file).await?,
        FunctionCommands::Package { spec_file } => package_actor(spec_file).await?,
    }
    Ok(())
}

pub(crate) async fn build_actor(spec_file: String) -> anyhow::Result<()> {
    let spec_file_path = std::fs::canonicalize(std::path::PathBuf::from(spec_file.clone()))?;
    let cargo_project_path = spec_file_path.parent().unwrap().to_path_buf();

    let function_spec: edgeless_config::actor_class::EdgelessActorClass = if spec_file_path.extension().unwrap() == "json" {
        serde_json::from_str(&std::fs::read_to_string(spec_file.clone()).unwrap()).unwrap()
    } else {
        match edgeless_config::load(spec_file_path).unwrap() {
            edgeless_config::LoadResult::ActorClass(a) => a,
            _ => {
                panic!("Can't Spawn Function as Workflow");
            }
        }
    };
    // let function_spec: edgeless_config::actor_class::EdgelessActorClass =
    //     serde_json::from_str(&std::fs::read_to_string(spec_file.clone())?)?;

    let out_file = cargo_project_path
        .join(format!("{}.wasm", function_spec.id))
        .to_str()
        .unwrap()
        .to_string();

    let mut port_features: Vec<_> = function_spec.inputs.keys().map(|i| format!("input_{i}")).collect();
    port_features.append(&mut function_spec.outputs.keys().map(|o| format!("output_{o}")).collect());

    match edgeless_build::wasm::rust_to_wasm(cargo_project_path.to_str().unwrap().to_string(), port_features, true, false) {
        Ok(result_file) => {
            std::fs::copy(result_file, out_file).unwrap();
        }
        Err(e) => panic!("{}", e.to_string()),
    }

    Ok(())
}

pub(crate) async fn package_actor(spec_file: String) -> anyhow::Result<()> {
    log::info!("{spec_file:?}");
    let spec_file_path = std::fs::canonicalize(std::path::PathBuf::from(spec_file.clone()))?;
    let cargo_project_path = spec_file_path.parent().unwrap().to_path_buf();

    let function_spec: edgeless_config::actor_class::EdgelessActorClass = if spec_file_path.extension().unwrap() == "json" {
        serde_json::from_str(&std::fs::read_to_string(spec_file.clone()).unwrap()).unwrap()
    } else {
        match edgeless_config::load(spec_file_path).unwrap() {
            edgeless_config::LoadResult::ActorClass(a) => {
                std::fs::write(cargo_project_path.join("function.json"), serde_json::to_vec(&a).unwrap()).unwrap();
                a
            }
            _ => {
                panic!("Can't Spawn Function as Workflow");
            }
        }
    };

    log::info!("{function_spec:?}");

    let out_file = cargo_project_path
        .join(format!("{}.tar.gz", function_spec.id))
        .to_str()
        .unwrap()
        .to_string();

    let packaged = edgeless_build::rust::package_rust(cargo_project_path.to_str().unwrap().to_string())?;
    std::fs::copy(packaged, out_file).unwrap();
    Ok(())
}
