// SPDX-FileCopyrightText: © 2026 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
#![no_std]

use edgeless_function::*;

struct DemoFilter;

edgeless_function::generate!(DemoFilter);

impl DemoFilterAPI<'_> for DemoFilter {
    type EFT_EVAL_MOCK_SENSOR_VALUE = edgeless_function_types::eval::MockSensorValue;

    fn handle_cast_data_in(_src: InstanceId, test_msg: Self::EFT_EVAL_MOCK_SENSOR_VALUE) {
        log::info!("Received data: value: {}", test_msg.value);
        if test_msg.value <= 25.0 {
            cast_accepted_out(&test_msg);
        } else {
            cast_rejected_out(&test_msg);
        }
    }

    fn handle_internal(_data: &[u8]) {
        log::info!("Handle internal called.");
    }

    fn handle_init(_payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();

        log::info!("Actor started.");
    }

    fn handle_stop() {
        log::info!("Actor stopped.");
    }
}
