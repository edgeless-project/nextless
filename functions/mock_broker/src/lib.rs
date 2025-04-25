// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
use edgeless_function::*;

struct MockBroker;

edgeless_function::generate!(MockBroker);

impl MockBrokerAPI<'_> for MockBroker {
    type TEST = String;

    fn handle_cast_input1(_src: InstanceId, test_msg: String) {
        cast_output1(&test_msg);
    }
    fn handle_cast_input2(_src: InstanceId, test_msg: String) {
        cast_output2(&test_msg);
    }
    fn handle_cast_input3(_src: InstanceId, test_msg: String) {
        cast_output3(&test_msg);
    }
    fn handle_cast_input4(_src: InstanceId, test_msg: String) {
        cast_output4(&test_msg);
    }
    fn handle_cast_input5(_src: InstanceId, test_msg: String) {
        cast_output5(&test_msg);
    }
    fn handle_cast_input6(_src: InstanceId, test_msg: String) {
        cast_output6(&test_msg);
    }
    fn handle_cast_input7(_src: InstanceId, test_msg: String) {
        cast_output7(&test_msg);
    }
    fn handle_cast_input8(_src: InstanceId, test_msg: String) {
        cast_output8(&test_msg);
    }
    fn handle_cast_input9(_src: InstanceId, test_msg: String) {
        cast_output9(&test_msg);
    }
    fn handle_cast_input10(_src: InstanceId, test_msg: String) {
        cast_output10(&test_msg);
    }

    fn handle_internal(data: &[u8]) {
        log::info!("Mock Broker Wakeup");
    }

    fn handle_init(_payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        log::info!("Mock Broker Started");
    }

    fn handle_stop() {
        log::info!("Mock Producer Stopped");
    }
}
