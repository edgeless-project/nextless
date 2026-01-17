// SPDX-FileCopyrightText: © 2026 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
use edgeless_function::*;

struct DemoDashboard;

edgeless_function::generate!(DemoDashboard);

#[derive(Debug)]
struct DashboardState {
    values: std::collections::VecDeque<f64>,
}

static STATE: std::sync::OnceLock<std::sync::Mutex<DashboardState>> = std::sync::OnceLock::new();

impl DemoDashboardAPI<'_> for DemoDashboard {
    type EFT_EVAL_MOCK_SENSOR_VALUE = edgeless_function_types::eval::MockSensorValue;
    type EDGELESS_HTTP_REQUEST = edgeless_http::EdgelessHTTPRequest;
    type EDGELESS_HTTP_RESPONSE = edgeless_http::EdgelessHTTPResponse;

    fn handle_cast_data_in(_src: InstanceId, test_msg: Self::EFT_EVAL_MOCK_SENSOR_VALUE) {
        let sensor_node_id = uuid::Uuid::from_bytes(test_msg.sensor_id.node_id);
        let sensor_component_id = uuid::Uuid::from_bytes(test_msg.sensor_id.component_id);

        log::info!(
            "Received Value. Sequence Number: {}. Value: {}. Sensor Node ID: {}. Sensor Component ID: {}",
            test_msg.sequence_number,
            test_msg.value,
            sensor_node_id,
            sensor_component_id,
        );

        let mut lck = STATE.get().unwrap().lock().unwrap();

        if lck.values.len() == 10 {
            lck.values.pop_front();
        }
        lck.values.push_back(test_msg.value);
    }

    fn handle_call_http_fetch(_src: InstanceId, req: Self::EDGELESS_HTTP_REQUEST) -> Self::EDGELESS_HTTP_RESPONSE {
        log::info!("Demo Dashboard received fetch request");

        if req.path == "/values" {
            let data = fetch_values();

            let data_json = serde_json::to_vec(&data).unwrap();

            edgeless_http::EdgelessHTTPResponse {
                status: 200,
                body: Some(data_json),
                headers: std::collections::HashMap::<String, String>::new(),
            }
        } else {
            edgeless_http::EdgelessHTTPResponse {
                status: 404,
                body: Some(Vec::<u8>::from("Not Found")),
                headers: std::collections::HashMap::<String, String>::new(),
            }
        }
    }

    fn handle_internal(_data: &[u8]) {
        log::info!("Demo Dashboard handle_internal called.");
    }

    fn handle_init(_payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();

        STATE
            .set(std::sync::Mutex::new(DashboardState {
                values: std::collections::VecDeque::with_capacity(10),
            }))
            .unwrap();

        log::info!("Demo Dashboard started.");
    }

    fn handle_stop() {
        log::info!("Demo Dashboard stopped.");
    }
}

fn fetch_values() -> Vec<f64> {
    let lck = STATE.get().unwrap().lock().unwrap();

    lck.values.iter().map(|v| *v).collect()
}
