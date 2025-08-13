// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

use edgeless_function::*;

struct UnusedOutput;

edgeless_function::generate!(UnusedOutput);

impl UnusedOutputAPI<'_> for UnusedOutput {
    type EFT_EVAL_NUMBERED_TEST_MESSAGE = edgeless_function_types::eval::NumberedTestMessage;

    fn handle_cast_data_in(_src: InstanceId, test_msg: Self::EFT_EVAL_NUMBERED_TEST_MESSAGE) {
        log::info!(
            "Forwarder with unused output got message: ID: {}. Payload Size: {}. Would trigger unused ouput: {}",
            test_msg.sequence_number,
            test_msg.payload.len(),
            test_msg.sequence_number % 10 == 0,
        );

        if test_msg.sequence_number % 10 == 0 {
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
            cast_unused_out(&out_msg);
        } else {
            cast_data_out(&test_msg);
        }
    }

    fn handle_internal(_data: &[u8]) {
        log::info!("Forwarder with unused output: handle_internal called.");
    }

    fn handle_init(_payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();

        log::info!("Forwarder with unused output started.");
    }

    fn handle_stop() {
        log::info!("Forwarder with unused output stopped.");
    }
}
