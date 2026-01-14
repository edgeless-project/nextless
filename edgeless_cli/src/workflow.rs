// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use edgeless_api::controller::ControllerAPI;

#[derive(Debug, clap::Subcommand)]
pub(crate) enum WorkflowCommands {
    Start {
        spec_file: String,
        #[arg(short, long, default_value_t = String::from(""))]
        extra_images: String,
    },
    Stop {
        id: String,
    },
    List {},
}

#[derive(Debug, thiserror::Error)]
pub enum WorkflowError {
    #[error("Error with provided input file '{file}'")]
    InputError { file: String, source: anyhow::Error },
    #[error("Could not load application description.")]
    StarlarkConfigurationError(#[from] edgeless_config::ConfigError),
    #[error("Configuration's entrypoint is not an application description")]
    NotAnApplication,
    #[error("Received malformed workflow id.")]
    BadWorkflowId,
    #[error("Error including component '{actor_id}'.")]
    ComponentError { actor_id: String, source: ComponentError },
}

#[derive(Debug, thiserror::Error)]
pub enum ComponentError {
    #[error("Error with code file '{file}'.")]
    CodeFileError { file: String, source: anyhow::Error },
    #[error("Unsupported Dialect: '{0}'.")]
    UnsupportedDialect(String),
    #[error("Failed to include extra image: '{source_dialect}' -> '{dest_dialect}'.")]
    ExtraImageInclusion {
        source_dialect: String,
        dest_dialect: String,
        source: anyhow::Error,
    },
    #[error("Unsupported mapping for Port '{port_id}'")]
    UnsupportedPortMapping { port_id: String },
}

pub(crate) async fn workflow_command(command: WorkflowCommands, config: crate::CLiConfig) -> Result<(), WorkflowError> {
    let mut con_client = edgeless_api::grpc_impl::controller::ControllerAPIClient::new(&config.controller_url).await;
    let mut con_wf_client = con_client.workflow_instance_api();
    match command {
        WorkflowCommands::Start { spec_file, extra_images } => start_workflow(con_wf_client.as_mut(), spec_file, extra_images).await?,
        WorkflowCommands::Stop { id } => stop_workflow(con_wf_client.as_mut(), id).await?,
        WorkflowCommands::List {} => list_workflows(con_wf_client.as_mut()).await?,
    }
    Ok(())
}

pub async fn start_workflow(
    workflow_instance_client: &mut dyn edgeless_api::workflow_instance::WorkflowInstanceAPI,
    spec_file: String,
    extra_images: String,
) -> Result<(), WorkflowError> {
    println!("Attempting to start Workflow with config '{spec_file}'...");

    let p = std::path::PathBuf::from(spec_file.clone());

    let file_extension = p.extension().ok_or(WorkflowError::InputError {
        file: p.to_string_lossy().to_string(),
        source: anyhow::anyhow!("Could not determine file extensions."),
    })?;

    let workflow: edgeless_config::workflow::EdgelessWorkflow = if file_extension == "json" {
        let json_config_string = std::fs::read_to_string(spec_file.clone()).map_err(|e| WorkflowError::InputError {
            file: p.to_string_lossy().to_string(),
            source: e.into(),
        })?;

        serde_json::from_str(&json_config_string).map_err(|e| WorkflowError::InputError {
            file: p.to_string_lossy().to_string(),
            source: e.into(),
        })?
    } else {
        let config = edgeless_config::load(p).map_err(WorkflowError::from)?;

        match config {
            edgeless_config::LoadResult::Workflow(wf) => wf,
            _ => return Err(WorkflowError::NotAnApplication),
        }
    };

    let start_workflow_request = api_request_for_workflow(workflow, extra_images)?;

    let res = workflow_instance_client.start(start_workflow_request).await;
    match res {
        Ok(response) => {
            match &response {
                edgeless_api::workflow_instance::SpawnWorkflowResponse::ResponseError(err) => {
                    println!("{err:?}");
                }
                edgeless_api::workflow_instance::SpawnWorkflowResponse::WorkflowInstance(val) => {
                    println!("{}", val.workflow_id.workflow_id);
                }
            }
            log::info!("{response:?}")
        }
        Err(err) => println!("{err}"),
    }

    Ok(())
}

pub async fn stop_workflow(
    workflow_instance_client: &mut dyn edgeless_api::workflow_instance::WorkflowInstanceAPI,
    workflow_id: String,
) -> Result<(), WorkflowError> {
    println!("Attempting to stop Application with id '{workflow_id}'...");
    let parsed_id = uuid::Uuid::parse_str(&workflow_id).map_err(|_e| WorkflowError::BadWorkflowId)?;
    match workflow_instance_client
        .stop(edgeless_api::workflow_instance::WorkflowId { workflow_id: parsed_id })
        .await
    {
        Ok(_) => println!("Workflow Stopped"),
        Err(err) => println!("{err}"),
    }
    Ok(())
}

pub async fn list_workflows(workflow_instance_client: &mut dyn edgeless_api::workflow_instance::WorkflowInstanceAPI) -> Result<(), WorkflowError> {
    match workflow_instance_client.list(edgeless_api::workflow_instance::WorkflowId::none()).await {
        Ok(instances) => {
            for instance in instances.iter() {
                println!("workflow: {}", instance.workflow_id);
                for function in instance.node_mapping.iter() {
                    println!("\t{function:?}");
                }
            }
        }
        Err(err) => println!("{err}"),
    }
    Ok(())
}

fn api_request_for_workflow(
    workflow: edgeless_config::workflow::EdgelessWorkflow,
    extra_images: String,
) -> Result<edgeless_api::workflow_instance::SpawnWorkflowRequest, WorkflowError> {
    let actors = workflow
        .actors
        .into_iter()
        .map(|func_spec| api_function_for_actor(func_spec.clone(), extra_images.clone()).map_err(|e| (func_spec.id, e)))
        .collect::<Result<Vec<edgeless_api::workflow_instance::WorkflowFunction>, (String, ComponentError)>>()
        .map_err(|(function_id, e)| WorkflowError::ComponentError {
            actor_id: function_id,
            source: e,
        })?;

    let resources = workflow
        .resources
        .into_iter()
        .map(|resource_spec| api_resource_for_resource(resource_spec.clone()).map_err(|e| (resource_spec.id, e)))
        .collect::<Result<Vec<edgeless_api::workflow_instance::WorkflowResource>, (String, ComponentError)>>()
        .map_err(|(function_id, e)| WorkflowError::ComponentError {
            actor_id: function_id,
            source: e,
        })?;

    Ok(edgeless_api::workflow_instance::SpawnWorkflowRequest {
        workflow_functions: actors,
        workflow_resources: resources,
        workflow_egress_proxies: Vec::new(),
        workflow_ingress_proxies: Vec::new(),
        annotations: workflow.annotations.clone(),
    })
}

fn api_function_for_actor(
    func_spec: edgeless_config::actor::EdgelessActorGen<edgeless_config::port::PortGen<edgeless_config::port::Mapping>>,
    extra_images: String,
) -> Result<edgeless_api::workflow_instance::WorkflowFunction, ComponentError> {
    Ok(edgeless_api::workflow_instance::WorkflowFunction {
        name: func_spec.id,
        behavior: api_behavior_for_actor_class(func_spec.klass, extra_images)?,
        output_mapping: func_spec
            .outputs
            .iter()
            .map(|(port_id, mapping)| {
                Ok((
                    edgeless_api::function_instance::PortId(port_id.clone()),
                    parse_port_mapping(&mapping.mapping).ok_or(ComponentError::UnsupportedPortMapping { port_id: port_id.clone() })?,
                ))
            })
            .collect::<Result<std::collections::HashMap<_, _>, ComponentError>>()?,
        input_mapping: func_spec
            .inputs
            .iter()
            .map(|(port_id, mapping)| {
                Ok((
                    edgeless_api::function_instance::PortId(port_id.clone()),
                    parse_port_mapping(&mapping.mapping).ok_or(ComponentError::UnsupportedPortMapping { port_id: port_id.clone() })?,
                ))
            })
            .collect::<Result<std::collections::HashMap<_, _>, ComponentError>>()?,
        annotations: func_spec.annotations.into_iter().collect(),
    })
}

fn api_behavior_for_actor_class(
    class: edgeless_config::actor_class::EdgelessActorClass,
    extra_images: String,
) -> Result<edgeless_api::behavior::Behavior, ComponentError> {
    let dialect = match class.code_type.as_str() {
        "WASM_BASE" => edgeless_api::node_registration::RuntimeType {
            base_type: "WASM".to_string(),
            features: Vec::new(),
        },
        "WASM_WGPU" => edgeless_api::node_registration::RuntimeType {
            base_type: "WASM".to_string(),
            features: vec!["WGPU".to_string()],
        },
        "RUST_BASE" => edgeless_api::node_registration::RuntimeType {
            base_type: "RUST".to_string(),
            features: Vec::new(),
        },
        "RUST_WGPU" => edgeless_api::node_registration::RuntimeType {
            base_type: "RUST".to_string(),
            features: vec!["WGPU".to_string()],
        },
        "RUST_NO_STD" => edgeless_api::node_registration::RuntimeType {
            base_type: "RUST".to_string(),
            features: vec!["NO_STD".to_string()],
        },
        _ => return Err(ComponentError::UnsupportedDialect(class.code_type.clone())),
    };

    let behavior_id = edgeless_api::behavior::BehaviorId {
        id: class.id.clone(),
        version: class.version.clone(),
    };

    let input_ports: std::collections::BTreeMap<_, _> = class
        .inputs
        .iter()
        .map(|(port_id, port_spec)| {
            (
                edgeless_api::function_instance::PortId(port_id.clone()),
                edgeless_api::function_instance::Port {
                    id: edgeless_api::function_instance::PortId(port_id.clone()),
                    method: match port_spec.method {
                        edgeless_config::port_class::Method::Call => edgeless_api::function_instance::PortMethod::Call,
                        edgeless_config::port_class::Method::Cast => edgeless_api::function_instance::PortMethod::Cast,
                    },
                    data_type: edgeless_api::function_instance::PortDataType(port_spec.data_type.clone()),
                    return_data_type: port_spec.return_data_type.clone().map(edgeless_api::function_instance::PortDataType),
                },
            )
        })
        .collect();

    let output_ports: std::collections::BTreeMap<_, _> = class
        .outputs
        .iter()
        .map(|(port_id, port_spec)| {
            (
                edgeless_api::function_instance::PortId(port_id.clone()),
                edgeless_api::function_instance::Port {
                    id: edgeless_api::function_instance::PortId(port_id.clone()),
                    method: match port_spec.method {
                        edgeless_config::port_class::Method::Call => edgeless_api::function_instance::PortMethod::Call,
                        edgeless_config::port_class::Method::Cast => edgeless_api::function_instance::PortMethod::Cast,
                    },
                    data_type: edgeless_api::function_instance::PortDataType(port_spec.data_type.clone()),
                    return_data_type: port_spec.return_data_type.clone().map(edgeless_api::function_instance::PortDataType),
                },
            )
        })
        .collect();

    let main_image = match class.code.clone() {
        Some(image_file) => {
            let code = std::fs::read(&image_file.path).map_err(|e| ComponentError::CodeFileError {
                file: image_file.path.clone(),
                source: e.into(),
            })?;
            let image = edgeless_api::behavior::BehaviorImage {
                behavior_image_id: edgeless_api::behavior::BehaviorImageId {
                    behaviour_id: behavior_id.clone(),
                    enabled_ports: edgeless_api::behavior::EnabledPorts {
                        enabled_inputs: input_ports.keys().cloned().collect(),
                        enabled_outputs: output_ports.keys().cloned().collect(),
                    },
                    dialect_type: dialect.clone(),
                },
                image: code,
            };

            Some(image)
        }
        None => None,
    };

    let extra_images: Vec<_> = {
        if let Some(main_image) = &main_image {
            // TODO: Cleanup extra image passing; This was added as part of a quick&dirty experiment
            if !extra_images.is_empty() && ["RUST"].contains(&dialect.base_type.as_str()) {
                extra_images
                    .split(",")
                    .map(|extra_image_base_type| collect_extra_image(extra_image_base_type, &class, &main_image.behavior_image_id))
                    .collect::<Result<Vec<_>, ComponentError>>()?
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        }
    };

    Ok(edgeless_api::behavior::Behavior {
        spec: edgeless_api::behavior::BehaviorSpec {
            behavior_id: behavior_id.clone(),
            input_ports: input_ports.clone(),
            output_ports: output_ports.clone(),
            inner_structure: class.inner_structure.iter().fold(
                std::collections::BTreeMap::<edgeless_api::function_instance::MappingNode, Vec<edgeless_api::function_instance::MappingNode>>::new(),
                |mut acc, mapping| {
                    let key = match &mapping.source {
                        edgeless_config::inner_structure::MappingNode::Port(port_id) => {
                            edgeless_api::function_instance::MappingNode::Port(edgeless_api::function_instance::PortId(port_id.clone()))
                        }
                        edgeless_config::inner_structure::MappingNode::SideEffect => edgeless_api::function_instance::MappingNode::SideEffect,
                    };

                    let data = acc.entry(key).or_default();

                    // I don't think there is a better way: https://stackoverflow.com/a/39803426
                    let mut tmp = std::collections::HashSet::<edgeless_api::function_instance::MappingNode>::from_iter(std::mem::take(data));

                    tmp.extend(mapping.dests.iter().map(|dest| match dest {
                        edgeless_config::inner_structure::MappingNode::Port(port_id) => {
                            edgeless_api::function_instance::MappingNode::Port(edgeless_api::function_instance::PortId(port_id.clone()))
                        }
                        edgeless_config::inner_structure::MappingNode::SideEffect => edgeless_api::function_instance::MappingNode::SideEffect,
                    }));

                    *data = tmp.into_iter().collect();
                    acc
                },
            ),
        },
        main_image: main_image,
        extra_images: extra_images,
    })
}

fn collect_extra_image(
    extra_image_base_type: &str,
    class: &edgeless_config::actor_class::EdgelessActorClass,
    base_image_id: &edgeless_api::behavior::BehaviorImageId,
) -> Result<edgeless_api::behavior::BehaviorImage, ComponentError> {
    let res = match extra_image_base_type {
        "WASM" => collect_wasm_image_for_rust_base(class, base_image_id),
        _ => Err(anyhow::anyhow!("Unimplemented Transformation")),
    };

    res.map_err(|e| ComponentError::ExtraImageInclusion {
        source_dialect: base_image_id.dialect_type.base_type.clone(),
        dest_dialect: extra_image_base_type.to_string(),
        source: e,
    })
}

fn collect_wasm_image_for_rust_base(
    class: &edgeless_config::actor_class::EdgelessActorClass,
    base_image_id: &edgeless_api::behavior::BehaviorImageId,
) -> anyhow::Result<edgeless_api::behavior::BehaviorImage> {
    let Some(rust_file) = class.code.clone() else {
        return Err(anyhow::anyhow!("Missing base image."));
    };

    let rust_file_path = std::path::PathBuf::from(rust_file.path);
    let dir = rust_file_path
        .parent()
        .ok_or(anyhow::anyhow!("Failed to get parent: {}", rust_file_path.to_string_lossy()))?;

    let base_name = rust_file_path
        .file_prefix()
        .ok_or(anyhow::anyhow!("Failed to get basename: {}", rust_file_path.to_string_lossy()))?
        .to_str()
        .ok_or(anyhow::anyhow!("Failed to stringify base_name: {}", rust_file_path.to_string_lossy()))?;

    let wasm_code_path = dir.join(std::path::PathBuf::from(format!("{base_name}.wasm")));

    let wasm_code = std::fs::read(&wasm_code_path).map_err(|e| ComponentError::CodeFileError {
        file: wasm_code_path.to_string_lossy().to_string(),
        source: e.into(),
    })?;

    Ok(edgeless_api::behavior::BehaviorImage {
        behavior_image_id: edgeless_api::behavior::BehaviorImageId {
            behaviour_id: base_image_id.behaviour_id.clone(),
            enabled_ports: edgeless_api::behavior::EnabledPorts {
                enabled_inputs: base_image_id.enabled_ports.enabled_inputs.clone(),
                enabled_outputs: base_image_id.enabled_ports.enabled_outputs.clone(),
            },
            dialect_type: edgeless_api::node_registration::RuntimeType {
                base_type: "WASM".to_string(),
                features: base_image_id.dialect_type.features.clone(),
            },
        },
        image: wasm_code,
    })
}

fn api_resource_for_resource(
    res_spec: edgeless_config::resource::EdgelessResourceGen<edgeless_config::port::PortGen<edgeless_config::port::Mapping>>,
) -> Result<edgeless_api::workflow_instance::WorkflowResource, ComponentError> {
    Ok(edgeless_api::workflow_instance::WorkflowResource {
        name: res_spec.id,
        class_type: res_spec.klass.id,
        output_mapping: res_spec
            .outputs
            .iter()
            .map(|(port_id, mapping)| {
                Ok((
                    edgeless_api::function_instance::PortId(port_id.clone()),
                    parse_port_mapping(&mapping.mapping).ok_or(ComponentError::UnsupportedPortMapping { port_id: port_id.clone() })?,
                ))
            })
            .collect::<Result<std::collections::HashMap<_, _>, ComponentError>>()?,
        input_mapping: res_spec
            .inputs
            .iter()
            .map(|(port_id, mapping)| {
                Ok((
                    edgeless_api::function_instance::PortId(port_id.clone()),
                    parse_port_mapping(&mapping.mapping).ok_or(ComponentError::UnsupportedPortMapping { port_id: port_id.clone() })?,
                ))
            })
            .collect::<Result<std::collections::HashMap<_, _>, ComponentError>>()?,
        configurations: res_spec.configurations,
    })
}

fn parse_port_mapping(mapping: &edgeless_config::port::Mapping) -> Option<edgeless_api::workflow_instance::PortMapping> {
    match mapping {
        edgeless_config::port::Mapping::Direct(direct_target) => Some(edgeless_api::workflow_instance::PortMapping::DirectTarget(
            direct_target.target_component.clone(),
            edgeless_api::function_instance::PortId(direct_target.port.clone()),
        )),
        edgeless_config::port::Mapping::Any(targets) => Some(edgeless_api::workflow_instance::PortMapping::AnyOfTargets(
            targets
                .iter()
                .map(|t| (t.target_component.clone(), edgeless_api::function_instance::PortId(t.port.clone())))
                .collect(),
        )),
        edgeless_config::port::Mapping::All(targets) => Some(edgeless_api::workflow_instance::PortMapping::AllOfTargets(
            targets
                .iter()
                .map(|t| (t.target_component.clone(), edgeless_api::function_instance::PortId(t.port.clone())))
                .collect(),
        )),
        edgeless_config::port::Mapping::Topic(topic_target) => Some(edgeless_api::workflow_instance::PortMapping::Topic(topic_target.clone())),
        _ => None,
    }
}
