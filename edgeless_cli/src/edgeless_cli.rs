// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

pub mod actor;
pub mod description;
pub mod workflow;

use clap::Parser;

use actor::FunctionCommands;
use description::DescriptionCommands;
use workflow::WorkflowCommands;

#[derive(Debug, clap::Subcommand)]
enum Commands {
    // https://stackoverflow.com/a/73790382
    #[clap(visible_alias("application"))]
    /// Interact (e.g., start/stop) with (an) Application(s).
    Workflow {
        #[command(subcommand)]
        workflow_command: WorkflowCommands,
    },
    #[clap(visible_alias("actor"))]
    /// Interact (e.g., build) with an Actor.
    Function {
        #[command(subcommand)]
        function_command: FunctionCommands,
    },
    /// Translate Starlark to json.
    Description {
        #[command(subcommand)]
        description_command: DescriptionCommands,
    },
}

#[derive(Debug, clap::Parser)]
#[command(long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,
    #[arg(short, long, default_value_t = String::from("cli.toml"))]
    config_file: String,
    #[arg(short, long, default_value_t = String::from(""))]
    template: String,
}

#[derive(serde::Deserialize)]
struct CLiConfig {
    controller_url: String,
}

#[derive(Debug, thiserror::Error)]
enum CliError {
    #[error("Could not load the cli configuration file '{file}'")]
    ConfigurationError { file: String, source: anyhow::Error },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let args = Args::parse();
    if !args.template.is_empty() {
        edgeless_api::util::create_template(&args.template, edgeless_cli_default_conf().as_str())?;
        return Ok(());
    }

    let conf_str = std::fs::read_to_string(&args.config_file).map_err(|e| CliError::ConfigurationError {
        file: args.config_file.clone(),
        source: e.into(),
    })?;
    let conf = toml::from_str(&conf_str).map_err(|e| CliError::ConfigurationError {
        file: args.config_file.clone(),
        source: e.into(),
    })?;

    match args.command {
        None => log::debug!("Bye"),
        Some(x) => match x {
            Commands::Workflow { workflow_command } => workflow::workflow_command(workflow_command, conf).await?,
            Commands::Function { function_command } => actor::actor_command(function_command, conf).await?,
            Commands::Description { description_command } => description::description_command(description_command, conf).await?,
        },
    }
    Ok(())
}

pub fn edgeless_cli_default_conf() -> String {
    String::from(
        r##"controller_url = "http://127.0.0.1:7001"
"##,
    )
}
