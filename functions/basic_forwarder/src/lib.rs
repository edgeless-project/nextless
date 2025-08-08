// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
extern "C" {
    fn eval_sleep(delay_ms: u64);
}

fn fake_work(delay_ms: u64) {
    unsafe {
        eval_sleep(delay_ms);
    }
}

use edgeless_function::*;

struct BasicForwarder;

static FAKE_DELAY_MS: std::sync::OnceLock<u64> = std::sync::OnceLock::new();

edgeless_function::generate!(BasicForwarder);

impl BasicForwarderAPI<'_> for BasicForwarder {
    type EFT_EVAL_NUMBERED_TEST_MESSAGE = edgeless_function_types::eval::NumberedTestMessage;

    fn handle_cast_data_in(_src: InstanceId, test_msg: Self::EFT_EVAL_NUMBERED_TEST_MESSAGE) {
        let delay = *FAKE_DELAY_MS.get().unwrap();
        log::info!(
            "Forwarder Got Message: ID: {}. Payload Size: {}. Delay: {}.",
            test_msg.sequence_number,
            test_msg.payload.len(),
            delay
        );

        if delay > 0 {
            fake_work(delay);
        }

        cast_data_out(&test_msg);
    }

    fn handle_internal(_data: &[u8]) {
        log::info!("Basic Forwarder Internal Called");
    }

    fn handle_init(payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        let delay = if let Some(raw_delay) = payload {
            core::str::from_utf8(raw_delay).unwrap().parse::<u64>().unwrap()
        } else {
            0
        };

        FAKE_DELAY_MS.set(delay).unwrap();

        log::info!("Basic Forwarder Started.");
    }

    fn handle_stop() {
        log::info!("Basic Forwarder Stopped");
    }
}
