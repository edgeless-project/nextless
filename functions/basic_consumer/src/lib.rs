// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
use edgeless_function::*;

struct BasicConsumer;

edgeless_function::generate!(BasicConsumer);

impl BasicConsumerAPI<'_> for BasicConsumer {
    type EFT_EVAL_NUMBERED_TEST_MESSAGE = edgeless_function_types::eval::NumberedTestMessage;

    fn handle_cast_data_in(_src: InstanceId, test_msg: Self::EFT_EVAL_NUMBERED_TEST_MESSAGE) {
        log::info!(
            "Consumer Got Message. Sequence Number: {}. Payload Size: {}.",
            test_msg.sequence_number,
            test_msg.payload.len()
        );
    }

    fn handle_internal(_data: &[u8]) {
        log::info!("Consumer handle_internal called.");
    }

    fn handle_init(_payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        log::info!("Consumer started.");
    }

    fn handle_stop() {
        log::info!("Consumer stopped.");
    }
}
