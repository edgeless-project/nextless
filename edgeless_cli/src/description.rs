// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

#[derive(Debug, clap::Subcommand)]
pub(crate) enum DescriptionCommands {
    Transpile { file: String },
}

#[derive(Debug, thiserror::Error)]
pub enum DescriptionError {
    #[error("Error with provided input file '{file}'")]
    InputError { file: String, source: anyhow::Error },
    #[error("Error creating output file '{file}'")]
    OutputError { file: String, source: anyhow::Error },
    #[error("Could not load application description.")]
    StarlarkConfigurationError(#[from] edgeless_config::ConfigError),
}

pub(crate) async fn description_command(command: DescriptionCommands, _config: crate::CLiConfig) -> Result<(), DescriptionError> {
    match command {
        DescriptionCommands::Transpile { file } => {
            let input_path = std::path::PathBuf::from(file.clone());
            let parent = input_path
                .parent()
                .ok_or(DescriptionError::InputError {
                    file: file.clone(),
                    source: anyhow::anyhow!("Could not get file parent."),
                })?
                .to_path_buf();
            let input_configuration = edgeless_config::load(std::path::PathBuf::from(file.clone()))?;

            // Try-block, cf. https://stackoverflow.com/a/72118745
            (|| match input_configuration {
                edgeless_config::LoadResult::Workflow(wf) => {
                    let output_confguration = serde_json::to_vec(&wf)?;
                    let output_path = parent.join(std::path::PathBuf::from("workflow.json"));
                    std::fs::write(output_path, &output_confguration)
                }
                edgeless_config::LoadResult::ActorClass(a) => {
                    let output_configuration = serde_json::to_vec(&a)?;
                    let output_path = parent.join(std::path::PathBuf::from("function.json"));
                    std::fs::write(output_path, &output_configuration)
                }
            })()
            .map_err(|e| DescriptionError::OutputError {
                file: file.clone(),
                source: e.into(),
            })
        }
    }
}
