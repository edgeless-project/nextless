// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

use edgeless_function::*;

struct UnusedInput;

edgeless_function::generate!(UnusedInput);

impl UnusedInputAPI<'_> for UnusedInput {
    type EFT_EVAL_NUMBERED_TEST_MESSAGE = edgeless_function_types::eval::NumberedTestMessage;

    fn handle_cast_data_in(_src: InstanceId, test_msg: Self::EFT_EVAL_NUMBERED_TEST_MESSAGE) {
        log::info!(
            "Got Expected Message: ID: {}. Payload Size: {}.",
            test_msg.sequence_number,
            test_msg.payload.len()
        );
        cast_data_out(&test_msg);
    }

    fn handle_cast_unused_in(_src: InstanceId, test_msg: Self::EFT_EVAL_NUMBERED_TEST_MESSAGE) {
        log::info!(
            "Got Message on Input that is supposed to be unused!. Message ID {}. Payload Size: {}. ",
            test_msg.sequence_number,
            test_msg.payload.len(),
        );

        let data = include_bytes!("padding.dat");
        let mut num_a = 0;

        let dynamic_comparison = 'a' as u8 + ((test_msg.sequence_number % 3) as u8);

        for b in data {
            if *b == dynamic_comparison {
                num_a += 1;
            }
        }

        let num_str = format!("{}", num_a);
        let new_payload = String::from_utf8(vec!['a' as u8; (test_msg.payload.len() - num_str.len()) as usize]).unwrap() + &num_str;

        let out_msg = edgeless_function_types::eval::NumberedTestMessage {
            sequence_number: test_msg.sequence_number,
            payload: new_payload,
        };

        cast_data_out(&out_msg);
    }

    fn handle_internal(_data: &[u8]) {
        log::info!("Basic Forwarder Internal Called");
    }

    fn handle_init(_payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();

        log::info!("Basic Forwarder Started.");
    }

    fn handle_stop() {
        log::info!("Basic Forwarder Stopped");
    }
}
