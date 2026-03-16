// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use crate::base_runtime::RuntimeAPI;
use crate::dataplane::core::CallRet;
use crate::dataplane::handle::DataplaneHandle;
use crate::telemetry::telemetry_events::TelemetryEvent;
use edgeless_api::function_instance::InstanceId;

#[derive(Clone)]
struct MockTelemetryHandle {
    sender: tokio::sync::mpsc::UnboundedSender<TestTelemetryEvent>,
}

struct TestTelemetryEvent {
    event: crate::telemetry::telemetry_events::TelemetryEvent,
    #[allow(dead_code)]
    tags: std::collections::BTreeMap<String, String>,
}

impl TestTelemetryEvent {
    fn is_function_instantiate(&self) -> bool {
        matches!(self.event, TelemetryEvent::FunctionInstantiate(_))
    }

    fn is_function_init(&self) -> bool {
        matches!(self.event, TelemetryEvent::FunctionInit(_))
    }

    fn is_function_log(&self) -> bool {
        matches!(self.event, TelemetryEvent::FunctionLogEntry(_, _, _))
    }

    fn is_function_invocation_completed(&self) -> bool {
        matches!(self.event, TelemetryEvent::FunctionInvocationCompleted { .. })
    }

    fn is_function_stop(&self) -> bool {
        matches!(self.event, TelemetryEvent::FunctionStop(_))
    }

    fn is_function_exit(&self) -> bool {
        matches!(self.event, TelemetryEvent::FunctionExit(_))
    }

    fn is_message_received(&self) -> bool {
        matches!(self.event, TelemetryEvent::MessageReceived(_))
    }
}

impl crate::telemetry::telemetry_events::TelemetryHandleAPI for MockTelemetryHandle {
    fn observe(&mut self, event: crate::telemetry::telemetry_events::TelemetryEvent, event_tags: std::collections::BTreeMap<String, String>) {
        self.sender.send(TestTelemetryEvent { event, tags: event_tags }).unwrap();
    }
    fn fork(&mut self, _child_tags: std::collections::BTreeMap<String, String>) -> Box<dyn crate::telemetry::telemetry_events::TelemetryHandleAPI> {
        Box::new(MockTelemetryHandle { sender: self.sender.clone() })
    }
}

struct MockStateMananger {
    output_mocks: std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<uuid::Uuid, String>>>,
    state_set_sender: tokio::sync::mpsc::UnboundedSender<(uuid::Uuid, String)>,
}

#[async_trait::async_trait]
impl crate::state_management::StateManagerAPI for MockStateMananger {
    async fn get_handle(
        &mut self,
        _state_policy: edgeless_api::function_instance::StatePolicy,
        state_id: uuid::Uuid,
    ) -> Box<dyn crate::state_management::StateHandleAPI> {
        Box::new(MockStateHandle {
            state_id,
            output_mocks: self.output_mocks.clone(),
            state_set_sender: self.state_set_sender.clone(),
        })
    }
}

struct MockStateHandle {
    state_id: uuid::Uuid,
    output_mocks: std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<uuid::Uuid, String>>>,
    state_set_sender: tokio::sync::mpsc::UnboundedSender<(uuid::Uuid, String)>,
}

#[async_trait::async_trait]
impl crate::state_management::StateHandleAPI for MockStateHandle {
    async fn get(&mut self) -> Option<String> {
        self.output_mocks.lock().await.get(&self.state_id).cloned()
    }

    async fn set(&mut self, serialized_state: String) {
        self.state_set_sender.send((self.state_id, serialized_state)).unwrap();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn basic_lifecycle() {
    let node_id = uuid::Uuid::new_v4();
    let instance_id = edgeless_api::function_instance::InstanceId::new(node_id);

    let state_manager = Box::new(crate::state_management::StateManager::new().await);
    let dataplane_provider = crate::dataplane::handle::DataplaneProvider::new(node_id, "http://127.0.0.1:7002".to_string(), None).await;

    let (telemetry_mock_sender, mut telemetry_mock_receiver) = tokio::sync::mpsc::unbounded_channel::<TestTelemetryEvent>();
    let telemetry_handle = Box::new(MockTelemetryHandle {
        sender: telemetry_mock_sender,
    });

    let (mut client, mut rt_task) = crate::base_runtime::runtime::create::<
        super::function_instance::WASMFunctionInstance,
        crate::base_runtime::function_instance_runner::FunctionInstanceRunner<super::function_instance::WASMFunctionInstance>,
    >(dataplane_provider, state_manager, telemetry_handle);

    tokio::spawn(async move { rt_task.run().await });

    let spawn_req = edgeless_api::function_instance::SpawnFunctionRequest {
        instance_id,
        code: edgeless_api::function_instance::FunctionClassSpecification {
            function_class_id: "EXAMPLE_1".to_string(),
            function_class_type: "WASM".to_string(),
            function_class_version: "0.1".to_string(),
            function_class_code: include_bytes!("fixtures/messaging_test.wasm").to_vec(),
            function_class_outputs: std::collections::HashMap::new(),
            function_class_inputs: std::collections::HashMap::new(),
            function_class_inner_structure: std::collections::HashMap::new(),
        },
        input_mapping: std::collections::HashMap::new(),
        output_mapping: std::collections::HashMap::new(),
        annotations: std::collections::HashMap::new(),
        state_specification: edgeless_api::function_instance::StateSpecification {
            state_id: instance_id.function_id,
            state_policy: edgeless_api::function_instance::StatePolicy::Transient,
        },
    };

    assert!(telemetry_mock_receiver.try_recv().is_err());

    let _res = client.start(spawn_req).await;

    // wait for lifetime events created during spawn

    let instantiate_res = tokio::time::timeout(tokio::time::Duration::from_secs(2), telemetry_mock_receiver.recv()).await;
    assert!(instantiate_res.unwrap().unwrap().is_function_instantiate());

    let instantiate_res = tokio::time::timeout(tokio::time::Duration::from_secs(2), telemetry_mock_receiver.recv()).await;
    assert!(instantiate_res.unwrap().unwrap().is_function_log());

    let instantiate_res = tokio::time::timeout(tokio::time::Duration::from_secs(2), telemetry_mock_receiver.recv()).await;
    assert!(instantiate_res.unwrap().unwrap().is_function_init());

    assert!(telemetry_mock_receiver.try_recv().is_err());

    let stop_res = client.stop(instance_id).await;
    assert!(stop_res.is_ok());

    let instantiate_res = tokio::time::timeout(tokio::time::Duration::from_secs(2), telemetry_mock_receiver.recv()).await;
    assert!(instantiate_res.unwrap().unwrap().is_function_log());

    let instantiate_res = tokio::time::timeout(tokio::time::Duration::from_secs(2), telemetry_mock_receiver.recv()).await;
    assert!(instantiate_res.unwrap().unwrap().is_function_stop());

    let instantiate_res = tokio::time::timeout(tokio::time::Duration::from_secs(2), telemetry_mock_receiver.recv()).await;
    assert!(instantiate_res.unwrap().unwrap().is_function_exit());
}

async fn messaging_test_setup() -> (
    crate::base_runtime::runtime::RuntimeClient,
    InstanceId,
    DataplaneHandle,
    InstanceId,
    DataplaneHandle,
    InstanceId,
    tokio::sync::mpsc::UnboundedReceiver<TestTelemetryEvent>,
) {
    // shared?
    let node_id = uuid::Uuid::new_v4();
    let instance_id = edgeless_api::function_instance::InstanceId::new(node_id);

    let state_manager = Box::new(crate::state_management::StateManager::new().await);
    let mut dataplane_provider = crate::dataplane::handle::DataplaneProvider::new(node_id, "http://127.0.0.1:7002".to_string(), None).await;

    // shared insert
    let test_peer_fid = edgeless_api::function_instance::InstanceId::new(node_id);
    let test_peer_handle = dataplane_provider.get_handle_for(test_peer_fid, None).await;

    let next_fid = edgeless_api::function_instance::InstanceId::new(node_id);
    let next_handle = dataplane_provider.get_handle_for(next_fid, None).await;
    // end shared insert

    let (telemetry_mock_sender, mut telemetry_mock_receiver) = tokio::sync::mpsc::unbounded_channel::<TestTelemetryEvent>();
    let telemetry_handle = Box::new(MockTelemetryHandle {
        sender: telemetry_mock_sender,
    });

    let (mut client, mut rt_task) = crate::base_runtime::runtime::create::<
        super::function_instance::WASMFunctionInstance,
        crate::base_runtime::function_instance_runner::FunctionInstanceRunner<super::function_instance::WASMFunctionInstance>,
    >(dataplane_provider, state_manager, telemetry_handle);

    tokio::spawn(async move { rt_task.run().await });

    let spawn_req = edgeless_api::function_instance::SpawnFunctionRequest {
        instance_id,
        code: edgeless_api::function_instance::FunctionClassSpecification {
            function_class_id: "EXAMPLE_1".to_string(),
            function_class_type: "WASM".to_string(),
            function_class_version: "0.1".to_string(),
            function_class_code: include_bytes!("fixtures/messaging_test.wasm").to_vec(),
            function_class_outputs: std::collections::HashMap::new(),
            function_class_inputs: std::collections::HashMap::new(),
            function_class_inner_structure: std::collections::HashMap::new(),
        },
        input_mapping: std::collections::HashMap::new(),
        output_mapping: std::collections::HashMap::from([
            (
                edgeless_api::function_instance::PortId("test_cast".to_string()),
                edgeless_api::common::Output::Single(next_fid, edgeless_api::function_instance::PortId("test_cast".to_string())),
            ),
            (
                edgeless_api::function_instance::PortId("test_call".to_string()),
                edgeless_api::common::Output::Single(next_fid, edgeless_api::function_instance::PortId("test_call".to_string())),
            ),
        ]),
        annotations: std::collections::HashMap::new(),
        state_specification: edgeless_api::function_instance::StateSpecification {
            state_id: instance_id.function_id,
            state_policy: edgeless_api::function_instance::StatePolicy::Transient,
        },
    };

    assert!(telemetry_mock_receiver.try_recv().is_err());

    let res = client.start(spawn_req).await;
    assert!(res.is_ok());

    let timeout_t_r = tokio::time::timeout(tokio::time::Duration::from_secs(2), telemetry_mock_receiver.recv()).await;
    assert!(timeout_t_r.unwrap().unwrap().is_function_instantiate());
    let timeout_t_r = tokio::time::timeout(tokio::time::Duration::from_secs(2), telemetry_mock_receiver.recv()).await;
    assert!(timeout_t_r.unwrap().unwrap().is_function_log());
    let timeout_t_r = tokio::time::timeout(tokio::time::Duration::from_secs(2), telemetry_mock_receiver.recv()).await;
    assert!(timeout_t_r.unwrap().unwrap().is_function_init());
    assert!(telemetry_mock_receiver.try_recv().is_err());

    (
        client,
        instance_id,
        test_peer_handle,
        test_peer_fid,
        next_handle,
        next_fid,
        telemetry_mock_receiver,
    )
}

// test input (host-> function): cast
// We assume this works after this test and trigger the different outputs using casts.
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn messaging_cast_raw_input() {
    let (_, instance_id, mut test_peer_handle, _test_peer_fid, _next_handle, _next_fid, mut telemetry_mock_receiver) = messaging_test_setup().await;
    test_peer_handle
        .send(
            instance_id,
            edgeless_api::function_instance::PortId("test_cast_input".to_string()),
            "some_message".as_bytes(),
            opentelemetry::Context::new(),
        )
        .await;

    let timeout_t_r = tokio::time::timeout(tokio::time::Duration::from_secs(2), telemetry_mock_receiver.recv()).await;
    assert!(timeout_t_r.unwrap().unwrap().is_message_received());
    let timeout_t_r = tokio::time::timeout(tokio::time::Duration::from_secs(2), telemetry_mock_receiver.recv()).await;
    assert!(timeout_t_r.unwrap().unwrap().is_function_log());
    let timeout_t_r = tokio::time::timeout(tokio::time::Duration::from_secs(2), telemetry_mock_receiver.recv()).await;
    assert!(timeout_t_r.unwrap().unwrap().is_function_invocation_completed());
    assert!(telemetry_mock_receiver.try_recv().is_err());
}

// test output (i.e. the method available to the function): cast
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn messaging_cast_raw_output() {
    let (_, instance_id, mut test_peer_handle, _test_peer_fid, _next_handle, _next_fid, mut telemetry_mock_receiver) = messaging_test_setup().await;

    test_peer_handle
        .send(
            instance_id,
            edgeless_api::function_instance::PortId("test_cast_input".to_string()),
            "test_cast_raw_output".as_bytes(),
            opentelemetry::Context::new(),
        )
        .await;
    let timeout_t_r = tokio::time::timeout(tokio::time::Duration::from_secs(2), telemetry_mock_receiver.recv()).await;
    assert!(timeout_t_r.unwrap().unwrap().is_message_received());

    let timeout_t_r = tokio::time::timeout(tokio::time::Duration::from_secs(2), telemetry_mock_receiver.recv()).await;
    assert!(timeout_t_r.unwrap().unwrap().is_function_invocation_completed());
    assert!(telemetry_mock_receiver.try_recv().is_err());

    let test_message = tokio::time::timeout(tokio::time::Duration::from_secs(2), test_peer_handle.receive_next())
        .await
        .unwrap();
    assert_eq!(test_message.source_id, instance_id);
    assert_eq!(
        test_message.message,
        crate::dataplane::core::Message::Cast("cast_raw_output".as_bytes().to_vec())
    );
}

// test output: call
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn messaging_call_raw_output() {
    let (_, instance_id, mut test_peer_handle, _test_peer_fid, _next_handle, _next_fid, mut telemetry_mock_receiver) = messaging_test_setup().await;

    test_peer_handle
        .send(
            instance_id,
            edgeless_api::function_instance::PortId("test_cast_input".to_string()),
            "test_call_raw_output".as_bytes(),
            opentelemetry::Context::new(),
        )
        .await;
    let timeout_t_r = tokio::time::timeout(tokio::time::Duration::from_secs(2), telemetry_mock_receiver.recv()).await;
    assert!(timeout_t_r.unwrap().is_some());

    // Operation Not Yet Completed
    assert!(telemetry_mock_receiver.try_recv().is_err());

    let test_message = test_peer_handle.receive_next().await;
    assert_eq!(test_message.source_id, instance_id);
    assert_eq!(
        test_message.message,
        crate::dataplane::core::Message::Call("call_raw_output".as_bytes().to_vec())
    );

    test_peer_handle
        .reply(test_message.source_id, test_message.channel_id, CallRet::NoReply)
        .await;

    let timeout_t_r = tokio::time::timeout(tokio::time::Duration::from_secs(2), telemetry_mock_receiver.recv()).await;
    assert!(timeout_t_r.unwrap().unwrap().is_function_invocation_completed());
    assert!(telemetry_mock_receiver.try_recv().is_err());
}

// test output: delayed_cast
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn messaging_delayed_cast_output() {
    let (_, instance_id, mut test_peer_handle, _test_peer_fid, mut next_handle, _next_fid, mut telemetry_mock_receiver) =
        messaging_test_setup().await;

    test_peer_handle
        .send(
            instance_id,
            edgeless_api::function_instance::PortId("test_cast_input".to_string()),
            "test_delayed_cast_output".as_bytes(),
            opentelemetry::Context::new(),
        )
        .await;
    let start = tokio::time::Instant::now();

    let test_message = next_handle.receive_next().await;
    assert!(start.elapsed() >= tokio::time::Duration::from_millis(100));

    assert_eq!(test_message.source_id, instance_id);
    assert_eq!(
        test_message.message,
        crate::dataplane::core::Message::Cast("delayed_cast_output".as_bytes().to_vec())
    );

    let timeout_t_r = tokio::time::timeout(tokio::time::Duration::from_secs(2), telemetry_mock_receiver.recv()).await;
    assert!(timeout_t_r.unwrap().unwrap().is_message_received());

    let timeout_t_r = tokio::time::timeout(tokio::time::Duration::from_secs(2), telemetry_mock_receiver.recv()).await;
    assert!(timeout_t_r.unwrap().unwrap().is_function_invocation_completed());

    assert!(telemetry_mock_receiver.try_recv().is_err());
}

// test output: cast
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn messaging_cast_output() {
    let (_, instance_id, mut test_peer_handle, _test_peer_fid, mut next_handle, _next_fid, mut telemetry_mock_receiver) =
        messaging_test_setup().await;

    test_peer_handle
        .send(
            instance_id,
            edgeless_api::function_instance::PortId("test_cast_input".to_string()),
            "test_cast_output".as_bytes(),
            opentelemetry::Context::new(),
        )
        .await;
    let timeout_t_r = tokio::time::timeout(tokio::time::Duration::from_secs(2), telemetry_mock_receiver.recv()).await;
    assert!(timeout_t_r.unwrap().unwrap().is_message_received());

    let timeout_t_r = tokio::time::timeout(tokio::time::Duration::from_secs(2), telemetry_mock_receiver.recv()).await;
    assert!(timeout_t_r.unwrap().unwrap().is_function_invocation_completed());

    assert!(telemetry_mock_receiver.try_recv().is_err());

    let test_message = next_handle.receive_next().await;
    assert_eq!(test_message.source_id, instance_id);
    assert_eq!(
        test_message.message,
        crate::dataplane::core::Message::Cast("cast_output".as_bytes().to_vec())
    );
}

// test output: call
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn messaging_call_output() {
    let (_, instance_id, mut test_peer_handle, _test_peer_fid, mut next_handle, _next_fid, mut telemetry_mock_receiver) =
        messaging_test_setup().await;

    test_peer_handle
        .send(
            instance_id,
            edgeless_api::function_instance::PortId("test_cast_input".to_string()),
            "test_call_output".as_bytes(),
            opentelemetry::Context::new(),
        )
        .await;
    let timeout_t_r = tokio::time::timeout(tokio::time::Duration::from_secs(2), telemetry_mock_receiver.recv()).await;
    assert!(timeout_t_r.unwrap().is_some());
    // This won't have completed here.
    assert!(telemetry_mock_receiver.try_recv().is_err());

    let test_message = next_handle.receive_next().await;
    assert_eq!(test_message.source_id, instance_id);
    assert_eq!(
        test_message.message,
        crate::dataplane::core::Message::Call("call_output".as_bytes().to_vec())
    );

    next_handle.reply(test_message.source_id, test_message.channel_id, CallRet::NoReply).await;

    let timeout_t_r = tokio::time::timeout(tokio::time::Duration::from_secs(2), telemetry_mock_receiver.recv()).await;
    assert!(timeout_t_r.unwrap().unwrap().is_function_invocation_completed());

    assert!(telemetry_mock_receiver.try_recv().is_err());
}

// test whether a function can be stopped while it is waiting for a call response
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn function_in_call_can_be_stopped() {
    let (mut client, instance_id, mut test_peer_handle, _test_peer_fid, mut next_handle, _next_fid, mut telemetry_mock_receiver) =
        messaging_test_setup().await;

    test_peer_handle
        .send(
            instance_id,
            edgeless_api::function_instance::PortId("test_cast_input".to_string()),
            "test_call_output".as_bytes(),
            opentelemetry::Context::new(),
        )
        .await;
    let timeout_t_r = tokio::time::timeout(tokio::time::Duration::from_secs(2), telemetry_mock_receiver.recv()).await;
    assert!(timeout_t_r.unwrap().is_some());
    // This won't have completed here.
    assert!(telemetry_mock_receiver.try_recv().is_err());

    let test_message = next_handle.receive_next().await;
    assert_eq!(test_message.source_id, instance_id);
    assert_eq!(
        test_message.message,
        crate::dataplane::core::Message::Call("call_output".as_bytes().to_vec())
    );

    assert!(telemetry_mock_receiver.try_recv().is_err());

    assert!(client.stop(instance_id).await.is_ok());
    let timeout_t_r = tokio::time::timeout(tokio::time::Duration::from_secs(2), telemetry_mock_receiver.recv()).await;
    assert!(timeout_t_r.unwrap().is_some());
}

// test call-interaction: Noreply
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn messaging_call_raw_input_noreply() {
    let (_, instance_id, mut test_peer_handle, _test_peer_fid, _next_handle, _next_fid, mut telemetry_mock_receiver) = messaging_test_setup().await;

    let ret = tokio::time::timeout(
        tokio::time::Duration::from_secs(2),
        test_peer_handle.call(
            instance_id,
            edgeless_api::function_instance::PortId("test_input_noreply".to_string()),
            "some_cast".as_bytes(),
            opentelemetry::Context::new(),
        ),
    )
    .await
    .unwrap();
    assert_eq!(ret, CallRet::NoReply);

    let timeout_t_r = tokio::time::timeout(tokio::time::Duration::from_secs(2), telemetry_mock_receiver.recv()).await;
    assert!(timeout_t_r.unwrap().unwrap().is_message_received());

    let timeout_t_r = tokio::time::timeout(tokio::time::Duration::from_secs(2), telemetry_mock_receiver.recv()).await;
    assert!(timeout_t_r.unwrap().unwrap().is_function_invocation_completed());

    assert!(telemetry_mock_receiver.try_recv().is_err());
}

// test call-interaction: Reply
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn messaging_call_raw_input_reply() {
    let (_, instance_id, mut test_peer_handle, _test_peer_fid, _next_handle, _next_fid, mut telemetry_mock_receiver) = messaging_test_setup().await;

    let ret = tokio::time::timeout(
        tokio::time::Duration::from_secs(2),
        test_peer_handle.call(
            instance_id,
            edgeless_api::function_instance::PortId("test_input_reply".to_string()),
            "test_ret".as_bytes(),
            opentelemetry::Context::new(),
        ),
    )
    .await
    .unwrap();
    assert_eq!(ret, CallRet::Reply("test_reply".as_bytes().to_vec()));

    let timeout_t_r = tokio::time::timeout(tokio::time::Duration::from_secs(2), telemetry_mock_receiver.recv()).await;
    assert!(timeout_t_r.unwrap().unwrap().is_message_received());

    let timeout_t_r = tokio::time::timeout(tokio::time::Duration::from_secs(2), telemetry_mock_receiver.recv()).await;
    assert!(timeout_t_r.unwrap().unwrap().is_function_invocation_completed());

    assert!(telemetry_mock_receiver.try_recv().is_err());
}

// test call-interaction: Error
// #[tokio::test]
// async fn messaging_call_raw_input_err() {
//     let (_, instance_id, mut test_peer_handle, _test_peer_fid, _next_handle, _next_fid, telemetry_mock_receiver) = messaging_test_setup().await;

//     let ret = test_peer_handle
//         .call(
//             instance_id.clone(),
//             edgeless_api::function_instance::PortId("test_cast_input".to_string()),
//             "test_err".to_string(),
//         )
//         .await;
//     assert_eq!(ret, CallRet::Err);

//     let telemetry_event = telemetry_mock_receiver.try_recv();
//     assert!(telemetry_event.is_ok());
//     let (telemetry_event, _tags) = telemetry_event.unwrap();
//     assert_eq!(
//         std::mem::discriminant(&telemetry_event),
//         std::mem::discriminant(&TelemetryEvent::FunctionInvocationCompleted(tokio::time::Duration::from_secs(1)))
//     );
//     assert!(telemetry_mock_receiver.try_recv().is_err());
// }

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn state_management() {
    let node_id = uuid::Uuid::new_v4();
    let instance_id = edgeless_api::function_instance::InstanceId::new(node_id);
    let fid2 = edgeless_api::function_instance::InstanceId::new(node_id);

    let output_mocks = std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));
    let (state_mock_sender, mut state_mock_receiver) = tokio::sync::mpsc::unbounded_channel::<(uuid::Uuid, String)>();
    let mock_state_manager = Box::new(MockStateMananger {
        state_set_sender: state_mock_sender,
        output_mocks: output_mocks.clone(),
    });

    let mut dataplane_provider = crate::dataplane::handle::DataplaneProvider::new(node_id, "http://127.0.0.1:7002".to_string(), None).await;

    let (telemetry_mock_sender, mut telemetry_mock_receiver) = tokio::sync::mpsc::unbounded_channel::<TestTelemetryEvent>();
    let telemetry_handle = Box::new(MockTelemetryHandle {
        sender: telemetry_mock_sender,
    });

    let test_peer_fid = edgeless_api::function_instance::InstanceId::new(node_id);
    let mut test_peer_handle = dataplane_provider.get_handle_for(test_peer_fid, None).await;

    let (mut client, mut rt_task) = crate::base_runtime::runtime::create::<
        super::function_instance::WASMFunctionInstance,
        crate::base_runtime::function_instance_runner::FunctionInstanceRunner<super::function_instance::WASMFunctionInstance>,
    >(dataplane_provider, mock_state_manager, telemetry_handle);

    tokio::spawn(async move { rt_task.run().await });

    let mut spawn_req = edgeless_api::function_instance::SpawnFunctionRequest {
        instance_id,
        code: edgeless_api::function_instance::FunctionClassSpecification {
            function_class_id: "EXAMPLE_1".to_string(),
            function_class_type: "WASM".to_string(),
            function_class_version: "0.1".to_string(),
            function_class_code: include_bytes!("fixtures/state_test.wasm").to_vec(),
            function_class_outputs: std::collections::HashMap::new(),
            function_class_inputs: std::collections::HashMap::new(),
            function_class_inner_structure: std::collections::HashMap::new(),
        },
        input_mapping: std::collections::HashMap::new(),
        output_mapping: std::collections::HashMap::new(),
        annotations: std::collections::HashMap::new(),
        state_specification: edgeless_api::function_instance::StateSpecification {
            state_id: instance_id.function_id,
            state_policy: edgeless_api::function_instance::StatePolicy::Transient,
        },
    };

    assert!(telemetry_mock_receiver.try_recv().is_err());

    let res = client.start(spawn_req.clone()).await;
    assert!(res.is_ok());

    let timeout_t_r = tokio::time::timeout(tokio::time::Duration::from_secs(2), telemetry_mock_receiver.recv()).await;
    assert!(timeout_t_r.unwrap().unwrap().is_function_instantiate());

    let test_telemetry_event = tokio::time::timeout(tokio::time::Duration::from_secs(2), telemetry_mock_receiver.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        test_telemetry_event.event,
        TelemetryEvent::FunctionLogEntry(
            crate::telemetry::telemetry_events::TelemetryLogLevel::Info,
            "state_test".to_string(),
            "no_state".to_string()
        )
    );

    let timeout_t_r = tokio::time::timeout(tokio::time::Duration::from_secs(2), telemetry_mock_receiver.recv()).await;
    assert!(timeout_t_r.unwrap().unwrap().is_function_init());

    assert!(telemetry_mock_receiver.try_recv().is_err());

    // trigger sync
    test_peer_handle
        .send(
            instance_id,
            edgeless_api::function_instance::PortId("test_cast_input".to_string()),
            "test_cast_raw_output".as_bytes(),
            opentelemetry::Context::new(),
        )
        .await;

    let (state_set_id, state_set_value) = tokio::time::timeout(tokio::time::Duration::from_secs(2), state_mock_receiver.recv())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(state_set_id, instance_id.function_id.clone());
    assert_eq!(state_set_value, "new_state".to_string());

    let timeout_t_r = tokio::time::timeout(tokio::time::Duration::from_secs(2), telemetry_mock_receiver.recv()).await;
    assert!(timeout_t_r.unwrap().unwrap().is_message_received());
    let timeout_t_r = tokio::time::timeout(tokio::time::Duration::from_secs(2), telemetry_mock_receiver.recv()).await;
    assert!(timeout_t_r.unwrap().unwrap().is_function_invocation_completed());
    assert!(telemetry_mock_receiver.try_recv().is_err());

    let res = client.stop(instance_id).await;
    assert!(res.is_ok());

    let timeout_t_r = tokio::time::timeout(tokio::time::Duration::from_secs(2), telemetry_mock_receiver.recv()).await;
    assert!(timeout_t_r.unwrap().unwrap().is_function_stop());
    let timeout_t_r = tokio::time::timeout(tokio::time::Duration::from_secs(2), telemetry_mock_receiver.recv()).await;
    assert!(timeout_t_r.unwrap().unwrap().is_function_exit());
    assert!(telemetry_mock_receiver.try_recv().is_err());

    // now we try starting with state

    output_mocks.lock().await.insert(instance_id.function_id, "existing_state".to_string());

    // TODO(raphaelhetzel) InstanceId reuse leads to problems that need to be fixed.
    spawn_req.instance_id = fid2;

    let res2 = client.start(spawn_req).await;
    assert!(res2.is_ok());

    let timeout_t_r = tokio::time::timeout(tokio::time::Duration::from_secs(2), telemetry_mock_receiver.recv()).await;
    assert!(timeout_t_r.unwrap().unwrap().is_function_instantiate());

    let test_telemetry_event = tokio::time::timeout(tokio::time::Duration::from_secs(2), telemetry_mock_receiver.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        test_telemetry_event.event,
        TelemetryEvent::FunctionLogEntry(
            crate::telemetry::telemetry_events::TelemetryLogLevel::Info,
            "edgeless_test_state".to_string(),
            "existing_state".to_string()
        )
    );

    let timeout_t_r = tokio::time::timeout(tokio::time::Duration::from_secs(2), telemetry_mock_receiver.recv()).await;
    assert!(timeout_t_r.unwrap().unwrap().is_function_init());
    assert!(telemetry_mock_receiver.try_recv().is_err());
}
