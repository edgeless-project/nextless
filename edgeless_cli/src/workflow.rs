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

pub(crate) async fn workflow_command(command: WorkflowCommands, config: crate::CLiConfig) -> Result<(), anyhow::Error> {
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
) -> Result<(), anyhow::Error> {
    log::debug!("Start Workflow");

    let p = std::path::PathBuf::from(spec_file.clone());
    let workflow: edgeless_config::workflow::EdgelessWorkflow = if p.extension().unwrap() == "json" {
        serde_json::from_str(&std::fs::read_to_string(spec_file.clone()).unwrap()).unwrap()
    } else {
        match edgeless_config::load(p).unwrap() {
            edgeless_config::LoadResult::Workflow(wf) => wf,
            _ => {
                panic!("Can't Spawn Function as Workflow");
            }
        }
    };
    let res = workflow_instance_client.start(api_request_for_workflow(workflow, extra_images)).await;
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
) -> Result<(), anyhow::Error> {
    let parsed_id = uuid::Uuid::parse_str(&workflow_id)?;
    match workflow_instance_client
        .stop(edgeless_api::workflow_instance::WorkflowId { workflow_id: parsed_id })
        .await
    {
        Ok(_) => println!("Workflow Stopped"),
        Err(err) => println!("{err}"),
    }
    Ok(())
}

pub async fn list_workflows(workflow_instance_client: &mut dyn edgeless_api::workflow_instance::WorkflowInstanceAPI) -> Result<(), anyhow::Error> {
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
) -> edgeless_api::workflow_instance::SpawnWorkflowRequest {
    edgeless_api::workflow_instance::SpawnWorkflowRequest {
        workflow_functions: workflow
            .actors
            .into_iter()
            .map(|func_spec| api_function_for_actor(func_spec, extra_images.clone()))
            .collect(),
        workflow_resources: workflow.resources.into_iter().map(api_resource_for_resource).collect(),
        workflow_egress_proxies: Vec::new(),
        workflow_ingress_proxies: Vec::new(),
        annotations: workflow.annotations.clone(),
    }
}

fn api_function_for_actor(
    func_spec: edgeless_config::actor::EdgelessActorGen<edgeless_config::port::PortGen<edgeless_config::port::Mapping>>,
    extra_images: String,
) -> edgeless_api::workflow_instance::WorkflowFunction {
    log::info!("{:?}", func_spec.klass.code.clone());

    edgeless_api::workflow_instance::WorkflowFunction {
        name: func_spec.id,
        behavior: api_behavior_for_actor_class(func_spec.klass, extra_images),
        output_mapping: func_spec
            .outputs
            .iter()
            .map(|(port_id, mapping)| {
                (
                    edgeless_api::function_instance::PortId(port_id.clone()),
                    parse_port_mapping(&mapping.mapping),
                )
            })
            .collect(),
        input_mapping: func_spec
            .inputs
            .iter()
            .map(|(port_id, mapping)| {
                (
                    edgeless_api::function_instance::PortId(port_id.clone()),
                    parse_port_mapping(&mapping.mapping),
                )
            })
            .collect(),
        annotations: func_spec.annotations.into_iter().collect(),
    }
}

fn api_behavior_for_actor_class(class: edgeless_config::actor_class::EdgelessActorClass, extra_images: String) -> edgeless_api::behavior::Behavior {
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
        _ => {
            panic!("Unusopported Dialect: {}", class.code_type.as_str())
        }
    };

    let behavior_id = edgeless_api::behavior::BehaviorId {
        id: class.id,
        version: class.version,
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

    edgeless_api::behavior::Behavior {
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
        main_image: Some(edgeless_api::behavior::BehaviorImage {
            behavior_image_id: edgeless_api::behavior::BehaviorImageId {
                behaviour_id: behavior_id.clone(),
                enabled_ports: edgeless_api::behavior::EnabledPorts {
                    enabled_inputs: input_ports.keys().cloned().collect(),
                    enabled_outputs: output_ports.keys().cloned().collect(),
                },
                dialect_type: dialect.clone(),
            },
            image: std::fs::read(class.code.clone().unwrap().path).unwrap(),
        }),
        // TODO: Cleanup; This is just a quick and dirty experiment but the whole file needs cleanup...
        extra_images: if ["RUST"].contains(&dialect.base_type.as_str()) {
            extra_images
                .split(",")
                .map(|i| match i {
                    "WASM" => {
                        let rust_file = std::path::PathBuf::from(class.code.clone().unwrap().path);
                        let dir = rust_file.parent().unwrap();
                        let base_name = rust_file.file_name().unwrap().to_str().unwrap().split(".").next().unwrap();

                        let wasm_code_path = dir.join(std::path::PathBuf::from(format!("{base_name}.wasm")));
                        let wasm_code = std::fs::read(wasm_code_path).unwrap();

                        edgeless_api::behavior::BehaviorImage {
                            behavior_image_id: edgeless_api::behavior::BehaviorImageId {
                                behaviour_id: behavior_id.clone(),
                                enabled_ports: edgeless_api::behavior::EnabledPorts {
                                    enabled_inputs: input_ports.keys().cloned().collect(),
                                    enabled_outputs: output_ports.keys().cloned().collect(),
                                },
                                dialect_type: edgeless_api::node_registration::RuntimeType {
                                    base_type: "WASM".to_string(),
                                    features: dialect.features.clone(),
                                },
                            },
                            image: wasm_code,
                        }
                    }
                    _ => {
                        panic!("Unsupported Extra Dialect")
                    }
                })
                .collect()
        } else {
            Vec::new()
        },
    }
}

fn api_resource_for_resource(
    res_spec: edgeless_config::resource::EdgelessResourceGen<edgeless_config::port::PortGen<edgeless_config::port::Mapping>>,
) -> edgeless_api::workflow_instance::WorkflowResource {
    edgeless_api::workflow_instance::WorkflowResource {
        name: res_spec.id,
        class_type: res_spec.klass.id,
        output_mapping: res_spec
            .outputs
            .iter()
            .map(|(port_id, mapping)| {
                (
                    edgeless_api::function_instance::PortId(port_id.clone()),
                    parse_port_mapping(&mapping.mapping),
                )
            })
            .collect(),
        input_mapping: res_spec
            .inputs
            .iter()
            .map(|(port_id, mapping)| {
                (
                    edgeless_api::function_instance::PortId(port_id.clone()),
                    parse_port_mapping(&mapping.mapping),
                )
            })
            .collect(),
        configurations: res_spec.configurations,
    }
}

fn parse_port_mapping(mapping: &edgeless_config::port::Mapping) -> edgeless_api::workflow_instance::PortMapping {
    match mapping {
        edgeless_config::port::Mapping::Direct(direct_target) => edgeless_api::workflow_instance::PortMapping::DirectTarget(
            direct_target.target_component.clone(),
            edgeless_api::function_instance::PortId(direct_target.port.clone()),
        ),
        edgeless_config::port::Mapping::Any(targets) => edgeless_api::workflow_instance::PortMapping::AnyOfTargets(
            targets
                .iter()
                .map(|t| (t.target_component.clone(), edgeless_api::function_instance::PortId(t.port.clone())))
                .collect(),
        ),
        edgeless_config::port::Mapping::All(targets) => edgeless_api::workflow_instance::PortMapping::AllOfTargets(
            targets
                .iter()
                .map(|t| (t.target_component.clone(), edgeless_api::function_instance::PortId(t.port.clone())))
                .collect(),
        ),
        edgeless_config::port::Mapping::Topic(topic_target) => edgeless_api::workflow_instance::PortMapping::Topic(topic_target.clone()),
        _ => {
            panic!("Bad Mapping");
        }
    }
}
