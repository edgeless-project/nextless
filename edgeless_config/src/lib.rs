// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

#![allow(clippy::needless_lifetimes)]

use std::str::FromStr;

pub mod actor;
pub mod actor_class;
pub mod files;
pub mod inner_structure;
pub mod port;
pub mod port_class;
pub mod resource;
pub mod resource_class;
pub mod workflow;

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Failed to interact with required file '{file}'.")]
    FileError { file: String, source: anyhow::Error },
    #[error("Dependency error processing file '{file}'.")]
    DependencyError { file: String, source: Box<ConfigError> },
    #[error("Config does not contain entrypoint 'el_main'.")]
    NoEntryPoint,
    #[error("Entrypoint 'el_main' is of unknown type.")]
    BadEntryPoint,
    #[error("Failed to parse file '{file}'.")]
    ParseError { file: String, source: anyhow::Error },
    #[error("Could not freeze file '{file}'.")]
    FreezeError { file: String, source: anyhow::Error },
    #[error("Error evaluating file '{file}'.")]
    EvalError { file: String, source: anyhow::Error },
}

#[derive(Debug)]
pub enum LoadResult {
    Workflow(crate::workflow::EdgelessWorkflow),
    ActorClass(crate::actor_class::EdgelessActorClass),
}

#[derive(Debug, starlark::any::ProvidesStaticType, Default)]
struct FileContext(std::path::PathBuf);

pub fn load(main_file: std::path::PathBuf) -> Result<LoadResult, ConfigError> {
    let m = load_module(&main_file)?;

    if let Ok(main) = m.get("el_main") {
        if let Ok(workflow) = main.clone().downcast::<crate::workflow::EdgelessWorkflow>() {
            return Ok(LoadResult::Workflow(workflow.as_ref().clone()));
        }

        if let Ok(actor) = main.downcast::<crate::actor_class::EdgelessActorClass>() {
            return Ok(LoadResult::ActorClass(actor.as_ref().clone()));
        }

        return Err(ConfigError::BadEntryPoint);
    } else {
        Err(ConfigError::NoEntryPoint)
    }
}

fn load_module(file: &std::path::PathBuf) -> Result<starlark::environment::FrozenModule, ConfigError> {
    // Uses lossy to_string and should only be used for error messages.
    let filepath_str_lossy = file.to_string_lossy().to_string();
    // Uses lossy to_string and should only be used for error messages.
    let filename_lossy = file
        .file_name()
        .ok_or(ConfigError::FileError {
            file: filepath_str_lossy.clone(),
            source: anyhow::anyhow!("Could not get filename."),
        })?
        .to_string_lossy()
        .to_string();

    let parent = file
        .parent()
        .ok_or(ConfigError::FileError {
            file: filepath_str_lossy.clone(),
            source: anyhow::anyhow!("Could not get file's parent."),
        })?
        .to_owned();

    let data = std::fs::read_to_string(file).map_err(|e| ConfigError::FileError {
        file: filepath_str_lossy.clone(),
        source: e.into(),
    })?;

    let ast =
        starlark::syntax::AstModule::parse(&filename_lossy, data, &starlark::syntax::Dialect::Standard).map_err(|e| ConfigError::ParseError {
            file: filepath_str_lossy.clone(),
            source: e.into_anyhow(),
        })?;

    let mut loads = std::collections::HashMap::new();

    for load in ast.loads() {
        let load_file = std::path::PathBuf::from_str(load.module_id).map_err(|e| ConfigError::DependencyError {
            file: filepath_str_lossy.clone(),
            source: Box::new(ConfigError::FileError {
                file: load.module_id.to_string(),
                source: anyhow::Error::from(e).context("Could parse module file path."),
            }),
        })?;

        let loaded_dependency = load_module(&parent.join(load_file)).map_err(|e| ConfigError::DependencyError {
            file: filepath_str_lossy.clone(),
            source: Box::new(e),
        })?;

        loads.insert(load.module_id.to_owned(), loaded_dependency);
    }

    let load_refs = loads.iter().map(|(k, v)| (k.as_str(), v)).collect();

    let loader = starlark::eval::ReturnFileLoader { modules: &load_refs };

    let globals = starlark::environment::GlobalsBuilder::extended_by(&[starlark::environment::LibraryExtension::Print])
        .with(crate::inner_structure::edgeless_inner_structure)
        .with(crate::actor_class::edgeless_actor_class)
        .with(crate::resource_class::edgeless_resource_class)
        .with(crate::actor::edgeless_actor)
        .with(crate::resource::edgeless_resource)
        .with(crate::port_class::edgeless_port_spec)
        .with(crate::workflow::edgeless_workflow)
        .with(crate::port::edgeless_port)
        .with(crate::files::file)
        .build();

    let module = starlark::environment::Module::new();
    let context = FileContext(file.canonicalize().map_err(|e| ConfigError::FileError {
        file: filepath_str_lossy.clone(),
        source: anyhow::Error::from(e).context("Failed to canonicalize."),
    })?);

    {
        let mut eval = starlark::eval::Evaluator::new(&module);
        eval.set_loader(&loader);
        eval.extra = Some(&context);
        eval.eval_module(ast, &globals).map_err(|e| ConfigError::EvalError {
            file: filepath_str_lossy.clone(),
            source: e.into_anyhow(),
        })?;
    }

    Ok(module.freeze().map_err(|e| ConfigError::FreezeError {
        file: filepath_str_lossy,
        source: e,
    })?)
}
