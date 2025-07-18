// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
use edgeless_function::*;

struct BasicConsumer;

extern "C" {
    fn eval_sleep(delay_ms: u64);
}

edgeless_function::generate!(BasicConsumer);

impl BasicConsumerAPI<'_> for BasicConsumer {
    type TEST = String;

    fn handle_cast_input_1(_src: InstanceId, test_msg: String) {
        log::info!("Consumer Got Message: {}", test_msg);
    }

    fn handle_internal(_data: &[u8]) {
        log::info!("Mock Consumer Internal Called");
    }

    fn handle_init(_payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        log::info!("Mock Consumer Started.");
    }

    fn handle_stop() {
        log::info!("Mock Consumer Stopped");
    }
}
