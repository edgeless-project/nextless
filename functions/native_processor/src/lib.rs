// SPDX-FileCopyrightText: © 2026 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

#![no_std]

use edgeless_function::*;

struct NativeProcessor;

edgeless_function::generate!(NativeProcessor);

impl NativeProcessorAPI<'_> for NativeProcessor {
    type EFT_EVAL_MOCK_SENSOR_VALUE = edgeless_function_types::eval::MockSensorValue;

    fn handle_cast_data_in(_src: InstanceId, test_msg: Self::EFT_EVAL_MOCK_SENSOR_VALUE) {
        let mut message = test_msg.clone();
        message.value = message.value * 2.0;
        let out_message = cast_data_out(&message);
    }

    fn handle_internal(_data: &[u8]) {
        log::info!("Handle internal called.");
    }

    fn handle_init(payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        log::info!("Actor started.");
    }

    fn handle_stop() {
        log::info!("Actor stopped.");
    }
}
