// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

#[derive(Debug, clap::Subcommand)]
pub(crate) enum DescriptionCommands {
    Transpile { file: String },
}

pub(crate) async fn description_command(command: DescriptionCommands, _config: crate::CLiConfig) -> Result<(), anyhow::Error> {
    match command {
        DescriptionCommands::Transpile { file } => {
            let path = std::path::PathBuf::from(file.clone());
            let parent = path.parent().unwrap().to_path_buf();
            let res = edgeless_config::load(std::path::PathBuf::from(file.clone())).unwrap();
            match res {
                edgeless_config::LoadResult::Workflow(wf) => {
                    let out = serde_json::to_string(&wf).unwrap();
                    std::fs::write(parent.join(std::path::PathBuf::from("workflow.json")), out.as_bytes()).unwrap();
                }
                edgeless_config::LoadResult::ActorClass(a) => {
                    let out = serde_json::to_string(&a).unwrap();
                    std::fs::write(parent.join(std::path::PathBuf::from("function.json")), out.as_bytes()).unwrap()
                }
            }
        }
    }
    Ok(())
}
