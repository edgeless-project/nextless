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
    type TEST_ID = String;

    fn handle_cast_in(_src: InstanceId, test_msg: Self::TEST_ID) {
        let delay = *FAKE_DELAY_MS.get().unwrap();
        log::info!("Forwarder Got Message: {}. Delay: {}.", test_msg, delay);

        if delay > 0 {
            fake_work(delay);
        }

        cast_out(&test_msg);
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
