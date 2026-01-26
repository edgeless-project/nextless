// SPDX-FileCopyrightText: © 2026 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
use edgeless_function::*;

struct DemoSensor;

edgeless_function::generate!(DemoSensor);

static COUNTER: std::sync::OnceLock<std::sync::Mutex<u64>> = std::sync::OnceLock::new();

impl DemoSensorAPI<'_> for DemoSensor {
    type EFT_EVAL_MOCK_SENSOR_VALUE = edgeless_function_types::eval::MockSensorValue;

    fn handle_internal(_data: &[u8]) {
        let mut state = COUNTER.get().unwrap().lock().unwrap();
        let iteration_id = *state;
        *state += 1;

        let instance_id = slf();

        let value = 15.0 + (iteration_id % 20) as f64;

        let payload = edgeless_function_types::eval::MockSensorValue {
            sequence_number: iteration_id,
            sensor_id: edgeless_function_types::eval::SensorId {
                node_id: instance_id.node_id,
                component_id: instance_id.component_id,
            },
            value: value,
        };

        log::info!("Sending Value: {}", value);
        cast_value(&payload);

        delayed_cast(2000, "self", &0u32.to_ne_bytes());
    }

    fn handle_init(_payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        COUNTER.set(std::sync::Mutex::new(0)).unwrap();
        log::info!("Stated. Start sending in 5s");
        delayed_cast(5000, "self", &0u32.to_ne_bytes());
    }

    fn handle_stop() {
        log::info!("Actor Stopped");
    }
}
