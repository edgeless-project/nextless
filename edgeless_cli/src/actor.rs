// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

#[derive(Debug, clap::Subcommand)]
pub(crate) enum FunctionCommands {
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

pub(crate) async fn actor_command(command: FunctionCommands, config: crate::CLiConfig) -> Result<(), anyhow::Error> {
    match command {
        FunctionCommands::Build { spec_file } => build_actor(spec_file).await?,
        FunctionCommands::Package { spec_file } => package_actor(spec_file).await?,
        FunctionCommands::Invoke {
            event_type,
            invocation_url,
            node_id,
            function_id,
            payload,
            target_port,
        } => {
            log::info!("invoking function: {event_type} {node_id} {function_id} {payload} {target_port}");
            let mut client = edgeless_api::grpc_impl::invocation::InvocationAPIClient::new(&invocation_url).await;
            let event = edgeless_api::invocation::Event {
                target: edgeless_api::function_instance::InstanceId {
                    node_id: uuid::Uuid::parse_str(&node_id)?,
                    function_id: uuid::Uuid::parse_str(&function_id)?,
                },
                source: edgeless_api::function_instance::InstanceId::none(),
                stream_id: 0,
                data: match event_type.as_str() {
                    "cast" => edgeless_api::invocation::EventData::Cast(payload.as_bytes().to_vec()),
                    _ => return Err(anyhow::anyhow!("invalid event type: {}", event_type)),
                },
                target_port: edgeless_api::function_instance::PortId(target_port),
                source_port: edgeless_api::function_instance::PortId("UNKNOWN".to_string()),
                context: opentelemetry::trace::SpanContext::empty_context(),
            };
            match edgeless_api::invocation::InvocationAPI::handle(&mut client, event).await {
                Ok(_) => println!("event casted"),
                Err(err) => return Err(anyhow::anyhow!("error casting the event: {}", err)),
            }
        }

        FunctionCommands::Get { function_name } => {
            let function_repository_conf = match config.function_repository {
                Some(conf) => conf,
                None => anyhow::bail!("function repository configuration section missing"),
            };

            let client = reqwest::Client::new();
            let response = client
                .get(function_repository_conf.url.to_string() + "/api/admin/function/" + function_name.as_str())
                .header(reqwest::header::ACCEPT, "application/json")
                .basic_auth(function_repository_conf.basic_auth_user, Some(function_repository_conf.basic_auth_pass))
                .send()
                .await
                .expect("failed to get response")
                .text()
                .await
                .expect("failed to get payload");

            println!("Successfully get function {response}");
        }

        FunctionCommands::Download { code_file_id } => {
            let function_repository_conf = match config.function_repository {
                Some(conf) => conf,
                None => anyhow::bail!("function repository configuration section missing"),
            };

            let client = reqwest::Client::new();
            let response = client
                .get(function_repository_conf.url.to_string() + "/api/admin/function/download/" + code_file_id.as_str())
                .header(reqwest::header::ACCEPT, "*/*")
                .basic_auth(function_repository_conf.basic_auth_user, Some(function_repository_conf.basic_auth_pass))
                .send()
                .await
                .expect("failed to get header");
            let status = response.status();
            println!("status code {status}");
            let header = response.headers().get("content-disposition").unwrap();

            let header_str = format!("{}{}", "Content-Disposition: ", header.to_str().unwrap());
            let (parsed, _) = mailparse::parse_header(header_str.as_bytes()).unwrap();
            let dis = mailparse::parse_content_disposition(&parsed.get_value());

            let downloadfilename = dis.params.get("filename").unwrap();

            println!("filename:\n{downloadfilename:?}");

            let body = response.bytes().await.expect("failed to download payload");

            let mut file = std::fs::File::create(downloadfilename)?;
            let mut content = std::io::Cursor::new(body);
            std::io::copy(&mut content, &mut file)?;

            println!("File downloaded successfully.");
        }

        FunctionCommands::Push { binary_name, function_type } => {
            let function_repository_conf = match config.function_repository {
                Some(conf) => conf,
                None => anyhow::bail!("function repository configuration section missing"),
            };

            let client = reqwest::Client::new();
            let file = tokio::fs::File::open(&binary_name).await?;

            // read file body stream
            let stream = tokio_util::codec::FramedRead::new(file, tokio_util::codec::BytesCodec::new());
            let file_body = reqwest::Body::wrap_stream(stream);

            //make form part of file
            let some_file = reqwest::multipart::Part::stream(file_body).file_name("binary"); // this is in curl -F "function_x86" in "file=@function_x86"

            //create the multipart form
            let form = reqwest::multipart::Form::new().part("file", some_file); // this is in curl -F "file"

            let response = client
                .post(function_repository_conf.url.to_string() + "/api/admin/function/upload")
                .header(reqwest::header::ACCEPT, "application/json")
                .basic_auth(
                    function_repository_conf.basic_auth_user.clone(),
                    Some(function_repository_conf.basic_auth_pass.clone()),
                )
                .multipart(form)
                .send()
                .await
                .expect("failed to get response");

            let json = response.json::<std::collections::HashMap<String, String>>().await?;
            println!("receive code_file_id {json:?}");

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
                .header(reqwest::header::ACCEPT, "application/json")
                .basic_auth(function_repository_conf.basic_auth_user, Some(function_repository_conf.basic_auth_pass))
                .json(&r)
                .send()
                .await
                .expect("failed to get response")
                .text()
                .await
                .expect("failed to get body");
            println!("post_response body: {post_response:?}");
        }
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
