// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
mod workflow_spec;

use clap::Parser;
use edgeless_api::{controller::ControllerAPI, workflow_instance::SpawnWorkflowResponse};

use mailparse::{parse_content_disposition, parse_header};
use reqwest::header::ACCEPT;
use reqwest::{multipart, Body, Client};
use std::collections::HashMap;
use std::io::Cursor;
use tokio::fs::File;
use tokio_util::codec::{BytesCodec, FramedRead};

#[derive(Debug, clap::Subcommand)]
enum WorkflowCommands {
    Start { spec_file: String },
    Stop { id: String },
    List {},
}

#[derive(Debug, clap::Subcommand)]
enum DescriptionCommands {
    Transpile { file: String },
}

#[derive(Debug, clap::Subcommand)]
enum FunctionCommands {
    Build {
        spec_file: String,
    },
    Package {
        spec_file: String,
    },
    Invoke {
        event_type: String,
        invocation_url: String,
        node_id: String,
        function_id: String,
        payload: String,
        target_port: String,
    },
    Get {
        function_name: String,
    },
    Download {
        code_file_id: String,
    },
    Push {
        binary_name: String,
        function_type: String,
    },
}

#[derive(Debug, clap::Subcommand)]
enum Commands {
    Workflow {
        #[command(subcommand)]
        workflow_command: WorkflowCommands,
    },
    Function {
        #[command(subcommand)]
        function_command: FunctionCommands,
    },
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
    function_repository: Option<FunctionRepositoryConfig>,
}

#[derive(serde::Deserialize)]
struct FunctionRepositoryConfig {
    pub url: String,
    pub basic_auth_user: String,
    pub basic_auth_pass: String,
}

pub fn edgeless_cli_default_conf() -> String {
    String::from(
        r##"controller_url = "http://127.0.0.1:7001"

#[function_repository]
#url = ""
#basic_auth_user = ""
#basic_auth_pass = ""
"##,
    )
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let args = Args::parse();
    if !args.template.is_empty() {
        edgeless_api::util::create_template(&args.template, edgeless_cli_default_conf().as_str())?;
        return Ok(());
    }

    match args.command {
        None => log::debug!("Bye"),
        Some(x) => match x {
            Commands::Workflow { workflow_command } => {
                if std::fs::metadata(&args.config_file).is_err() {
                    return Err(anyhow::anyhow!(
                        "configuration file does not exist or cannot be accessed: {}",
                        &args.config_file
                    ));
                }
                log::debug!("Got Config");
                let conf: CLiConfig = toml::from_str(&std::fs::read_to_string(args.config_file).unwrap()).unwrap();
                let mut con_client = edgeless_api::grpc_impl::controller::ControllerAPIClient::new(&conf.controller_url).await;
                let mut con_wf_client = con_client.workflow_instance_api();
                match workflow_command {
                    WorkflowCommands::Start { spec_file } => {
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
                        let res = con_wf_client
                            .start(edgeless_api::workflow_instance::SpawnWorkflowRequest {
                                workflow_functions: workflow
                                    .actors
                                    .into_iter()
                                    .map(|func_spec| {
                                        log::info!("{:?}", func_spec.klass.code.clone());
                                        let function_class_code = match func_spec.klass.code_type.as_str() {
                                            "RUST_WASM" => std::fs::read(func_spec.klass.code.unwrap().path).unwrap(),
                                            "RUST" => std::fs::read(func_spec.klass.code.unwrap().path).unwrap(),
                                            "CONTAINER" => func_spec.klass.code.unwrap().path.as_bytes().to_vec(),
                                            _ => panic!("unknown function class type: {}", func_spec.klass.id),
                                        };

                                        edgeless_api::workflow_instance::WorkflowFunction {
                                            name: func_spec.id,
                                            function_class_specification: edgeless_api::function_instance::FunctionClassSpecification {
                                                function_class_id: func_spec.klass.id,
                                                function_class_type: func_spec.klass.code_type,
                                                function_class_version: func_spec.klass.version,
                                                function_class_code,
                                                function_class_outputs: func_spec
                                                    .klass
                                                    .outputs
                                                    .iter()
                                                    .map(|(port_id, port_spec)| {
                                                        (
                                                            edgeless_api::function_instance::PortId(port_id.clone()),
                                                            edgeless_api::function_instance::Port {
                                                                id: edgeless_api::function_instance::PortId(port_id.clone()),
                                                                method: match port_spec.method {
                                                                    edgeless_config::port_class::Method::Call => {
                                                                        edgeless_api::function_instance::PortMethod::Call
                                                                    }
                                                                    edgeless_config::port_class::Method::Cast => {
                                                                        edgeless_api::function_instance::PortMethod::Cast
                                                                    }
                                                                },
                                                                data_type: edgeless_api::function_instance::PortDataType(port_spec.data_type.clone()),
                                                                return_data_type: port_spec
                                                                    .return_data_type
                                                                    .clone()
                                                                    .map(edgeless_api::function_instance::PortDataType),
                                                            },
                                                        )
                                                    })
                                                    .collect(),
                                                function_class_inputs: func_spec
                                                    .klass
                                                    .inputs
                                                    .iter()
                                                    .map(|(port_id, port_spec)| {
                                                        (
                                                            edgeless_api::function_instance::PortId(port_id.clone()),
                                                            edgeless_api::function_instance::Port {
                                                                id: edgeless_api::function_instance::PortId(port_id.clone()),
                                                                method: match port_spec.method {
                                                                    edgeless_config::port_class::Method::Call => {
                                                                        edgeless_api::function_instance::PortMethod::Call
                                                                    }
                                                                    edgeless_config::port_class::Method::Cast => {
                                                                        edgeless_api::function_instance::PortMethod::Cast
                                                                    }
                                                                },
                                                                data_type: edgeless_api::function_instance::PortDataType(port_spec.data_type.clone()),
                                                                return_data_type: port_spec
                                                                    .return_data_type
                                                                    .clone()
                                                                    .map(edgeless_api::function_instance::PortDataType),
                                                            },
                                                        )
                                                    })
                                                    .collect(),
                                                function_class_inner_structure: func_spec
                                                    .klass
                                                    .inner_structure
                                                    .iter()
                                                    .map(|mapping| {
                                                        (
                                                            match &mapping.source {
                                                                edgeless_config::inner_structure::MappingNode::Port(port_id) => {
                                                                    edgeless_api::function_instance::MappingNode::Port(
                                                                        edgeless_api::function_instance::PortId(port_id.clone()),
                                                                    )
                                                                }
                                                                edgeless_config::inner_structure::MappingNode::SideEffect => {
                                                                    edgeless_api::function_instance::MappingNode::SideEffect
                                                                }
                                                            },
                                                            mapping
                                                                .dests
                                                                .iter()
                                                                .map(|dest| match dest {
                                                                    edgeless_config::inner_structure::MappingNode::Port(port_id) => {
                                                                        edgeless_api::function_instance::MappingNode::Port(
                                                                            edgeless_api::function_instance::PortId(port_id.clone()),
                                                                        )
                                                                    }
                                                                    edgeless_config::inner_structure::MappingNode::SideEffect => {
                                                                        edgeless_api::function_instance::MappingNode::SideEffect
                                                                    }
                                                                })
                                                                .collect(),
                                                        )
                                                    })
                                                    .collect(),
                                            },
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
                                            annotations: func_spec.annotations,
                                        }
                                    })
                                    .collect(),
                                workflow_resources: workflow
                                    .resources
                                    .into_iter()
                                    .map(|res_spec| edgeless_api::workflow_instance::WorkflowResource {
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
                                    })
                                    .collect(),
                                workflow_egress_proxies: Vec::new(),
                                workflow_ingress_proxies: Vec::new(),
                                annotations: workflow.annotations.clone(),
                            })
                            .await;
                        match res {
                            Ok(response) => {
                                match &response {
                                    SpawnWorkflowResponse::ResponseError(err) => {
                                        println!("{:?}", err);
                                    }
                                    SpawnWorkflowResponse::WorkflowInstance(val) => {
                                        println!("{}", val.workflow_id.workflow_id);
                                    }
                                }
                                log::info!("{:?}", response)
                            }
                            Err(err) => println!("{}", err),
                        }
                    }
                    WorkflowCommands::Stop { id } => {
                        let parsed_id = uuid::Uuid::parse_str(&id)?;
                        match con_wf_client
                            .stop(edgeless_api::workflow_instance::WorkflowId { workflow_id: parsed_id })
                            .await
                        {
                            Ok(_) => println!("Workflow Stopped"),
                            Err(err) => println!("{}", err),
                        }
                    }
                    WorkflowCommands::List {} => match con_wf_client.list(edgeless_api::workflow_instance::WorkflowId::none()).await {
                        Ok(instances) => {
                            for instance in instances.iter() {
                                println!("workflow: {}", instance.workflow_id.to_string());
                                for function in instance.node_mapping.iter() {
                                    println!("\t{:?}", function);
                                }
                            }
                        }
                        Err(err) => println!("{}", err),
                    },
                }
            }
            Commands::Function { function_command } => match function_command {
                FunctionCommands::Build { spec_file } => {
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

                    let compiled = edgeless_build::rust_to_wasm(cargo_project_path.to_str().unwrap().to_string(), vec![], true, true)?;
                    std::fs::copy(compiled, out_file).unwrap();
                }

                FunctionCommands::Package { spec_file } => {
                    log::info!("{:?}", spec_file);
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

                    log::info!("{:?}", function_spec);

                    let out_file = cargo_project_path
                        .join(format!("{}.tar.gz", function_spec.id))
                        .to_str()
                        .unwrap()
                        .to_string();

                    let packaged = edgeless_build::package_rust(cargo_project_path.to_str().unwrap().to_string())?;
                    std::fs::copy(packaged, out_file).unwrap();
                }
                FunctionCommands::Invoke {
                    event_type,
                    invocation_url,
                    node_id,
                    function_id,
                    payload,
                    target_port,
                } => {
                    log::info!(
                        "invoking function: {} {} {} {} {}",
                        event_type,
                        node_id,
                        function_id,
                        payload,
                        target_port
                    );
                    let mut client = edgeless_api::grpc_impl::invocation::InvocationAPIClient::new(&invocation_url).await;
                    let event = edgeless_api::invocation::Event {
                        target: edgeless_api::function_instance::InstanceId {
                            node_id: uuid::Uuid::parse_str(&node_id)?,
                            function_id: uuid::Uuid::parse_str(&function_id)?,
                        },
                        source: edgeless_api::function_instance::InstanceId::none(),
                        stream_id: 0,
                        data: match event_type.as_str() {
                            "cast" => edgeless_api::invocation::EventData::Cast(payload),
                            _ => return Err(anyhow::anyhow!("invalid event type: {}", event_type)),
                        },
                        target_port: edgeless_api::function_instance::PortId(target_port),
                        context: opentelemetry::trace::SpanContext::empty_context()
                    };
                    match edgeless_api::invocation::InvocationAPI::handle(&mut client, event).await {
                        Ok(_) => println!("event casted"),
                        Err(err) => return Err(anyhow::anyhow!("error casting the event: {}", err)),
                    }
                }

                FunctionCommands::Get { function_name } => {
                    if std::fs::metadata(&args.config_file).is_err() {
                        return Err(anyhow::anyhow!(
                            "configuration file does not exist or cannot be accessed: {}",
                            &args.config_file
                        ));
                    }
                    log::debug!("Got Config");
                    let conf: CLiConfig = toml::from_str(&std::fs::read_to_string(args.config_file).unwrap()).unwrap();
                    let function_repository_conf = match conf.function_repository {
                        Some(conf) => conf,
                        None => anyhow::bail!("function repository configuration section missing"),
                    };

                    let client = Client::new();
                    let response = client
                        .get(function_repository_conf.url.to_string() + "/api/admin/function/" + function_name.as_str())
                        .header(ACCEPT, "application/json")
                        .basic_auth(function_repository_conf.basic_auth_user, Some(function_repository_conf.basic_auth_pass))
                        .send()
                        .await
                        .expect("failed to get response")
                        .text()
                        .await
                        .expect("failed to get payload");

                    println!("Successfully get function {}", response);
                }

                FunctionCommands::Download { code_file_id } => {
                    if std::fs::metadata(&args.config_file).is_err() {
                        return Err(anyhow::anyhow!(
                            "configuration file does not exist or cannot be accessed: {}",
                            &args.config_file
                        ));
                    }
                    log::debug!("Got Config");
                    let conf: CLiConfig = toml::from_str(&std::fs::read_to_string(args.config_file).unwrap()).unwrap();
                    let function_repository_conf = match conf.function_repository {
                        Some(conf) => conf,
                        None => anyhow::bail!("function repository configuration section missing"),
                    };

                    let client = Client::new();
                    let response = client
                        .get(function_repository_conf.url.to_string() + "/api/admin/function/download/" + code_file_id.as_str())
                        .header(ACCEPT, "*/*")
                        .basic_auth(function_repository_conf.basic_auth_user, Some(function_repository_conf.basic_auth_pass))
                        .send()
                        .await
                        .expect("failed to get header");
                    let status = response.status();
                    println!("status code {}", status);
                    let header = response.headers().get("content-disposition").unwrap();

                    let header_str = format!("{}{}", "Content-Disposition: ", header.to_str().unwrap());
                    let (parsed, _) = parse_header(header_str.as_bytes()).unwrap();
                    let dis = parse_content_disposition(&parsed.get_value());

                    let downloadfilename = dis.params.get("filename").unwrap();

                    println!("filename:\n{:?}", downloadfilename);

                    let body = response.bytes().await.expect("failed to download payload");

                    let mut file = std::fs::File::create(downloadfilename)?;
                    let mut content = Cursor::new(body);
                    std::io::copy(&mut content, &mut file)?;

                    println!("File downloaded successfully.");
                }

                FunctionCommands::Push { binary_name, function_type } => {
                    if std::fs::metadata(&args.config_file).is_err() {
                        return Err(anyhow::anyhow!(
                            "configuration file does not exist or cannot be accessed: {}",
                            &args.config_file
                        ));
                    }
                    log::debug!("Got Config");
                    let conf: CLiConfig = toml::from_str(&std::fs::read_to_string(&args.config_file).unwrap()).unwrap();
                    let function_repository_conf = match conf.function_repository {
                        Some(conf) => conf,
                        None => anyhow::bail!("function repository configuration section missing"),
                    };

                    let client = Client::new();
                    let file = File::open(&binary_name).await?;

                    // read file body stream
                    let stream = FramedRead::new(file, BytesCodec::new());
                    let file_body = Body::wrap_stream(stream);

                    //make form part of file
                    let some_file = multipart::Part::stream(file_body).file_name("binary"); // this is in curl -F "function_x86" in "file=@function_x86"

                    //create the multipart form
                    let form = multipart::Form::new().part("file", some_file); // this is in curl -F "file"

                    let response = client
                        .post(function_repository_conf.url.to_string() + "/api/admin/function/upload")
                        .header(ACCEPT, "application/json")
                        .basic_auth(
                            function_repository_conf.basic_auth_user.clone(),
                            Some(function_repository_conf.basic_auth_pass.clone()),
                        )
                        .multipart(form)
                        .send()
                        .await
                        .expect("failed to get response");

                    let json = response.json::<HashMap<String, String>>().await?;
                    println!("receive code_file_id {:?}", json);

                    let internal_id = &binary_name;
                    let r = serde_json::json!({

                        "function_type": function_type,
                        "id": internal_id,
                        "version": "0.1",
                        "code_file_id": json.get("id"), //get the id
                        "outputs": [  "success_cb",
                                      "failure_cb"
                                   ],
                    });

                    let post_response = client
                        .post(function_repository_conf.url.to_string() + "/api/admin/function")
                        .header(ACCEPT, "application/json")
                        .basic_auth(function_repository_conf.basic_auth_user, Some(function_repository_conf.basic_auth_pass))
                        .json(&r)
                        .send()
                        .await
                        .expect("failed to get response")
                        .text()
                        .await
                        .expect("failed to get body");
                    println!("post_response body: {:?}", post_response);
                }
            },
            Commands::Description { description_command } => match description_command {
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
            },
        },
    }
    Ok(())
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

// fn parse_config_workflow(cwf: edgeless_config::workflow::EdgelessWorkflow) -> {

// }

// fn parse_config_actor(
//     ca: edgeless_config::actor_class::EdgelessActorClass,
//     parent: &std::path::PathBuf,
// ) -> workflow_spec::WorkflowSpecFunctionClass {
//     workflow_spec::WorkflowSpecFunctionClass {
//         id: ca.id.clone(),
//         function_type: "RUST".to_string(),
//         version: ca.version,
//         code: Some(String::from_str(parent.join(std::path::PathBuf::from(format!("{}.tar.gz", ca.id))).to_str().unwrap()).unwrap()),
//         build: None,
//         outputs: ca.ouputs.into_iter().map(|(oid, o)| {
//             (
//                 oid,
//                 workflow_spec::PortDefinition {
//                     method: match o.method {
//                         edgeless_config::port_class::Method::Call => {
//                             workflow_spec::PortMethod::CALL
//                         },
//                         edgeless_config::port_class::Method::Cast => {
//                             workflow_spec::PortMethod::CAST
//                         }
//                     },
//                     data_type: o.data_type,
//                     return_data_type: o.return_data_type
//                 }
//             )
//         }).collect(),
//         inputs: ca.inputs.into_iter().map(|(oid, o)| {
//             (
//                 oid,
//                 workflow_spec::PortDefinition {
//                     method: match o.method {
//                         edgeless_config::port_class::Method::Call => {
//                             workflow_spec::PortMethod::CALL
//                         },
//                         edgeless_config::port_class::Method::Cast => {
//                             workflow_spec::PortMethod::CAST
//                         }
//                     },
//                     data_type: o.data_type,
//                     return_data_type: o.return_data_type
//                 }
//             )
//         }).collect(),
//         inner_structure: ca.inner_structure.into_iter().map(|cm| {
//             workflow_spec::Mapping {
//                 source: match cm.source {
//                     edgeless_config::inner_structure::MappingNode::Port(pid) => {
//                         workflow_spec::MappingNode::PORT(pid)
//                     },
//                     edgeless_config::inner_structure::MappingNode::SideEffect => {
//                         workflow_spec::MappingNode::SIDE_EFFECT
//                     }
//                 },
//                 dests: cm.dests.into_iter().map(|dest| match dest {
//                     edgeless_config::inner_structure::MappingNode::Port(pid) => {
//                         workflow_spec::MappingNode::PORT(pid)
//                     },
//                     edgeless_config::inner_structure::MappingNode::SideEffect => {
//                         workflow_spec::MappingNode::SIDE_EFFECT
//                     }
//                 }).collect()
//             }
//         }).collect(),
//     }
// }
