// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
use edgeless_function::*;

struct MockConsumer;

edgeless_function::generate!(MockConsumer);

impl MockConsumerAPI<'_> for MockConsumer {
    type TEST = String;

    fn handle_cast_input1(_src: InstanceId, test_msg: String) {
        log::info!("Consumer Got Message: {}", test_msg);
    }
    fn handle_cast_input2(_src: InstanceId, test_msg: String) {
        log::info!("Consumer Got Message: {}", test_msg);
    }
    fn handle_cast_input3(_src: InstanceId, test_msg: String) {
        log::info!("Consumer Got Message: {}", test_msg);
    }
    fn handle_cast_input4(_src: InstanceId, test_msg: String) {
        log::info!("Consumer Got Message: {}", test_msg);
    }
    fn handle_cast_input5(_src: InstanceId, test_msg: String) {
        log::info!("Consumer Got Message: {}", test_msg);
    }
    fn handle_cast_input6(_src: InstanceId, test_msg: String) {
        log::info!("Consumer Got Message: {}", test_msg);
    }
    fn handle_cast_input7(_src: InstanceId, test_msg: String) {
        log::info!("Consumer Got Message: {}", test_msg);
    }
    fn handle_cast_input8(_src: InstanceId, test_msg: String) {
        log::info!("Consumer Got Message: {}", test_msg);
    }
    fn handle_cast_input9(_src: InstanceId, test_msg: String) {
        log::info!("Consumer Got Message: {}", test_msg);
    }
    fn handle_cast_input10(_src: InstanceId, test_msg: String) {
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
