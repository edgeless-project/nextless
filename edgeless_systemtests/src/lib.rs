// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

#[cfg(test)]
mod tests {
    // use super::*;

    use edgeless_api::controller::ControllerAPI;
    use edgeless_api::workflow_instance::WorkflowInstanceAPI;

    async fn setup(
        num_domains: u32,
        num_nodes_per_domain: u32,
        redis_url: Option<&str>,
    ) -> (Vec<futures::future::AbortHandle>, Box<(dyn WorkflowInstanceAPI)>) {
        assert!(num_domains > 0);
        assert!(num_nodes_per_domain > 0);

        let mut handles = vec![];

        let address = "127.0.0.1";
        let mut port = 7001;
        let controller_url = format!("http://{}:{}", address, port);

        // Closure automatically incrementing the port number
        let mut next_port = || {
            port += 1;
            port
        };

        let (task, handle) = futures::future::abortable(edgeless_con::edgeless_con_main(edgeless_con::EdgelessConSettings {
            controller_url: controller_url.clone(),
            orchestrators: vec![],
        }));
        tokio::spawn(task);
        handles.push(handle);

        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        // The first node in each domain is also assigned a file-log resource.
        for node_i in 0..num_nodes_per_domain {
            let (task, handle) = futures::future::abortable(edgeless_node::edgeless_node_main(match node_i {
                0 => edgeless_node::EdgelessNodeSettings {
                    general: edgeless_node::EdgelessNodeGeneralSettings {
                        node_id: uuid::Uuid::new_v4(),
                        agent_url: format!("http://{}:{}", address, next_port()),
                        agent_url_announced: "".to_string(),
                        invocation_url: format!("http://{}:{}", address, next_port()),
                        invocation_url_announced: "".to_string(),
                        invocation_url_coap: None,
                        invocation_url_announced_coap: None,
                        metrics_url: format!("http://{}:{}", address, next_port()),
                        controller_url: controller_url.to_string(),
                    },
                    wasm_runtime: Some(edgeless_node::EdgelessNodeWasmRuntimeSettings { enabled: true }),
                    container_runtime: None,
                    resources: Some(edgeless_node::EdgelessNodeResourceSettings {
                        http_ingress_url: None,
                        http_ingress_provider: None,
                        http_egress_provider: None,
                        file_log_provider: Some("file-log-1".to_string()),
                        redis_provider: None,
                        dda_url: None,
                        dda_provider: None,
                        ollama_provider: None,
                    }),
                    user_node_capabilities: None,
                },
                _ => edgeless_node::EdgelessNodeSettings::new_without_resources(&controller_url, address, next_port(), next_port(), next_port()),
            }));
            tokio::spawn(task);
            handles.push(handle);
        }

        let mut con_client = edgeless_api::grpc_impl::controller::ControllerAPIClient::new(controller_url.as_str()).await;

        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

        (handles, con_client.workflow_instance_api())
    }

    async fn wf_list(client: &mut Box<(dyn WorkflowInstanceAPI)>) -> Vec<edgeless_api::workflow_instance::WorkflowInstance> {
        match client.list(edgeless_api::workflow_instance::WorkflowId::none()).await {
            Ok(instances) => instances,
            Err(_) => vec![],
        }
    }

    fn fixture_spec() -> edgeless_api::function_instance::FunctionClassSpecification {
        let out_1 = edgeless_api::function_instance::Port {
            id: edgeless_api::function_instance::PortId("out1".to_string()),
            method: edgeless_api::function_instance::PortMethod::Cast,
            data_type: edgeless_api::function_instance::PortDataType("String".to_string()),
            return_data_type: None,
        };

        let out_2 = edgeless_api::function_instance::Port {
            id: edgeless_api::function_instance::PortId("out2".to_string()),
            method: edgeless_api::function_instance::PortMethod::Cast,
            data_type: edgeless_api::function_instance::PortDataType("String".to_string()),
            return_data_type: None,
        };

        let out_err = edgeless_api::function_instance::Port {
            id: edgeless_api::function_instance::PortId("err".to_string()),
            method: edgeless_api::function_instance::PortMethod::Cast,
            data_type: edgeless_api::function_instance::PortDataType("String".to_string()),
            return_data_type: None,
        };

        let out_log = edgeless_api::function_instance::Port {
            id: edgeless_api::function_instance::PortId("log".to_string()),
            method: edgeless_api::function_instance::PortMethod::Cast,
            data_type: edgeless_api::function_instance::PortDataType("String".to_string()),
            return_data_type: None,
        };

        let in_1 = edgeless_api::function_instance::Port {
            id: edgeless_api::function_instance::PortId("in1".to_string()),
            method: edgeless_api::function_instance::PortMethod::Cast,
            data_type: edgeless_api::function_instance::PortDataType("String".to_string()),
            return_data_type: None,
        };

        edgeless_api::function_instance::FunctionClassSpecification {
            function_class_id: "system_test".to_string(),
            function_class_type: "RUST_WASM".to_string(),
            function_class_version: "0.1".to_string(),
            function_class_code: include_bytes!("fixtures/system_test.wasm").to_vec(),
            function_class_outputs: std::collections::HashMap::from([
                (edgeless_api::function_instance::PortId("out1".to_string()), out_1),
                (edgeless_api::function_instance::PortId("out2".to_string()), out_2),
                (edgeless_api::function_instance::PortId("err".to_string()), out_err),
                (edgeless_api::function_instance::PortId("log".to_string()), out_log),
            ]),
            function_class_inputs: std::collections::HashMap::from([(edgeless_api::function_instance::PortId("in1".to_string()), in_1)]),
            function_class_inner_structure: std::collections::HashMap::from([
                (
                    edgeless_api::function_instance::MappingNode::Port(edgeless_api::function_instance::PortId("in1".to_string())),
                    vec![
                        edgeless_api::function_instance::MappingNode::Port(edgeless_api::function_instance::PortId("out1".to_string())),
                        edgeless_api::function_instance::MappingNode::Port(edgeless_api::function_instance::PortId("out2".to_string())),
                        edgeless_api::function_instance::MappingNode::Port(edgeless_api::function_instance::PortId("err".to_string())),
                        edgeless_api::function_instance::MappingNode::Port(edgeless_api::function_instance::PortId("log".to_string())),
                    ],
                ),
                (
                    edgeless_api::function_instance::MappingNode::SideEffect,
                    vec![
                        edgeless_api::function_instance::MappingNode::Port(edgeless_api::function_instance::PortId("out1".to_string())),
                        edgeless_api::function_instance::MappingNode::Port(edgeless_api::function_instance::PortId("out2".to_string())),
                        edgeless_api::function_instance::MappingNode::Port(edgeless_api::function_instance::PortId("err".to_string())),
                        edgeless_api::function_instance::MappingNode::Port(edgeless_api::function_instance::PortId("log".to_string())),
                    ],
                ),
            ]),
        }
    }

    fn terminate(handles: Vec<futures::future::AbortHandle>) -> anyhow::Result<()> {
        for handle in handles {
            handle.abort();
        }
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    #[serial_test::serial]
    async fn system_test_single_domain_single_node() -> anyhow::Result<()> {
        let _ = env_logger::try_init();

        // Create the EDGELESS system.
        let (handles, mut client) = setup(1, 1, None).await;

        assert!(wf_list(&mut client).await.is_empty());

        // Create 10 workflows
        let mut workflow_ids = vec![];
        for _ in 0..10 {
            let res = client
                .start(edgeless_api::workflow_instance::SpawnWorkflowRequest {
                    workflow_functions: vec![edgeless_api::workflow_instance::WorkflowFunction {
                        name: "f1".to_string(),
                        function_class_specification: fixture_spec(),
                        output_mapping: std::collections::HashMap::new(),
                        input_mapping: std::collections::HashMap::new(),
                        annotations: std::collections::HashMap::new(),
                    }],
                    workflow_resources: vec![],
                    workflow_egress_proxies: vec![],
                    workflow_ingress_proxies: vec![],
                    annotations: std::collections::HashMap::new(),
                })
                .await;
            workflow_ids.push(match res {
                Ok(response) => match &response {
                    edgeless_api::workflow_instance::SpawnWorkflowResponse::ResponseError(err) => {
                        panic!("workflow rejected: {}", err)
                    }
                    edgeless_api::workflow_instance::SpawnWorkflowResponse::WorkflowInstance(val) => {
                        assert_eq!(1, val.node_mapping.len());
                        assert_eq!("f1", val.node_mapping[0].name);
                        assert_eq!(1, val.node_mapping[0].node_ids.len());
                        val.workflow_id.clone()
                    }
                },
                Err(err) => panic!("could not start the workflow: {}", err),
            });
        }

        assert_eq!(10, wf_list(&mut client).await.len());

        // Stop the workflows
        for workflow_id in workflow_ids {
            match client.stop(workflow_id).await {
                Ok(_) => {}
                Err(err) => panic!("could not stop the workflow: {}", err),
            }
        }

        // Stop a non-existing workflow
        match client
            .stop(edgeless_api::workflow_instance::WorkflowId {
                workflow_id: uuid::Uuid::new_v4(),
            })
            .await
        {
            Ok(_) => {}
            Err(err) => panic!("could not stop the workflow: {}", err),
        }
        assert!(wf_list(&mut client).await.is_empty());

        terminate(handles)
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    #[serial_test::serial]
    async fn system_test_single_domain_three_nodes() -> anyhow::Result<()> {
        let _ = env_logger::try_init();

        // Create the EDGELESS system.
        let (handles, mut client) = setup(1, 3, None).await;

        assert!(wf_list(&mut client).await.is_empty());

        let num_workflows = 3;

        let removeme_filename = |workflow_i| format!("removeme-{}.log", workflow_i);

        let cleanup = || {
            for workflow_i in 0..num_workflows {
                let _ = std::fs::remove_file(removeme_filename(workflow_i));
            }
        };

        // Create workflows
        let mut workflow_ids = vec![];
        cleanup();
        for workflow_i in 0..num_workflows {
            let res = client
                .start(edgeless_api::workflow_instance::SpawnWorkflowRequest {
                    workflow_functions: vec![
                        edgeless_api::workflow_instance::WorkflowFunction {
                            name: "f1".to_string(),
                            function_class_specification: fixture_spec(),
                            output_mapping: std::collections::HashMap::from([
                                (
                                    edgeless_api::function_instance::PortId("out1".to_string()),
                                    edgeless_api::workflow_instance::PortMapping::DirectTarget(
                                        "f2".to_string(),
                                        edgeless_api::function_instance::PortId("in1".to_string()),
                                    ),
                                ),
                                (
                                    edgeless_api::function_instance::PortId("out2".to_string()),
                                    edgeless_api::workflow_instance::PortMapping::DirectTarget(
                                        "f3".to_string(),
                                        edgeless_api::function_instance::PortId("in1".to_string()),
                                    ),
                                ),
                                (
                                    edgeless_api::function_instance::PortId("log".to_string()),
                                    edgeless_api::workflow_instance::PortMapping::DirectTarget(
                                        "log".to_string(),
                                        edgeless_api::function_instance::PortId("line".to_string()),
                                    ),
                                ),
                            ]),
                            input_mapping: std::collections::HashMap::new(),
                            annotations: std::collections::HashMap::from([("init-payload".to_string(), "8".to_string())]),
                        },
                        edgeless_api::workflow_instance::WorkflowFunction {
                            name: "f2".to_string(),
                            function_class_specification: fixture_spec(),
                            output_mapping: std::collections::HashMap::from([(
                                edgeless_api::function_instance::PortId("log".to_string()),
                                edgeless_api::workflow_instance::PortMapping::DirectTarget(
                                    "log".to_string(),
                                    edgeless_api::function_instance::PortId("line".to_string()),
                                ),
                            )]),
                            input_mapping: std::collections::HashMap::new(),
                            annotations: std::collections::HashMap::new(),
                        },
                        edgeless_api::workflow_instance::WorkflowFunction {
                            name: "f3".to_string(),
                            function_class_specification: fixture_spec(),
                            output_mapping: std::collections::HashMap::from([(
                                edgeless_api::function_instance::PortId("log".to_string()),
                                edgeless_api::workflow_instance::PortMapping::DirectTarget(
                                    "log".to_string(),
                                    edgeless_api::function_instance::PortId("line".to_string()),
                                ),
                            )]),
                            input_mapping: std::collections::HashMap::new(),
                            annotations: std::collections::HashMap::new(),
                        },
                    ],
                    workflow_resources: vec![edgeless_api::workflow_instance::WorkflowResource {
                        name: "log".to_string(),
                        class_type: "file-log".to_string(),
                        output_mapping: std::collections::HashMap::new(),
                        input_mapping: std::collections::HashMap::new(),
                        configurations: std::collections::HashMap::from([("filename".to_string(), removeme_filename(workflow_i))]),
                    }],
                    workflow_egress_proxies: vec![],
                    workflow_ingress_proxies: vec![],
                    annotations: std::collections::HashMap::new(),
                })
                .await;
            workflow_ids.push(match res {
                Ok(response) => match &response {
                    edgeless_api::workflow_instance::SpawnWorkflowResponse::ResponseError(err) => {
                        panic!("workflow rejected: {}", err)
                    }
                    edgeless_api::workflow_instance::SpawnWorkflowResponse::WorkflowInstance(val) => {
                        assert_eq!(4, val.node_mapping.len());
                        let mut expected_names = std::collections::HashSet::<String>::new();
                        // let mut expected_domains = std::collections::HashSet::<String>::new();
                        let mut actual_names = std::collections::HashSet::<String>::new();
                        // let mut actual_domains = std::collections::HashSet::<String>::new();

                        for i in 0..4 {
                            match i {
                                3 => expected_names.insert("log".to_string()),
                                _ => expected_names.insert(format!("f{}", i + 1)),
                            };
                            // expected_domains.insert("domain-0".to_string());
                            actual_names.insert(val.node_mapping[i].name.clone());
                            // actual_domains.insert(val.domain_mapping[i].domain_id.clone());
                        }
                        assert_eq!(expected_names, actual_names);
                        // assert_eq!(expected_domains, actual_domains);
                        val.workflow_id.clone()
                    }
                },
                Err(err) => panic!("could not start the workflow: {}", err),
            });
        }

        assert_eq!(num_workflows, wf_list(&mut client).await.len());

        // Wait until the log files have been filled.
        let mut not_done_yet: std::collections::HashSet<usize> = std::collections::HashSet::from_iter(0..num_workflows);
        let values_expected = std::collections::HashSet::<i32>::from([4, 7, 8]);
        for _ in 0..100 {
            for workflow_i in 0..num_workflows {
                let values_from_file: std::collections::HashSet<i32> = std::fs::read_to_string(removeme_filename(workflow_i))
                    .expect("could not read file")
                    .split('\n')
                    .filter_map(|x| x.parse::<i32>().ok())
                    .collect();
                if values_from_file == values_expected {
                    not_done_yet.remove(&workflow_i);
                }
            }
            if not_done_yet.is_empty() {
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }
        assert!(not_done_yet.is_empty(), "not all logs have been filled properly");

        // Stop the workflows
        for workflow_id in workflow_ids {
            match client.stop(workflow_id).await {
                Ok(_) => {}
                Err(err) => panic!("could not stop the workflow: {}", err),
            }
        }
        assert!(wf_list(&mut client).await.is_empty());

        cleanup();
        terminate(handles)
    }
}
