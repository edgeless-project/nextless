// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

#![no_std]

#[cfg(target_arch = "wasm32")]
extern "C" {
    fn eval_sleep(delay_ms: u64);
}

#[cfg(target_arch = "wasm32")]
fn fake_work(delay_ms: u64) {
    unsafe {
        eval_sleep(delay_ms);
    }
}
#[cfg(not(target_arch = "wasm32"))]
fn fake_work(delay_ms: u64) {
}

use edgeless_function::*;

struct NativeBenefitFwd;

static mut FAKE_DELAY_MS: u64 = 0;

edgeless_function::generate!(NativeBenefitFwd);

impl NativeBenefitFwdAPI<'_> for NativeBenefitFwd {
    type EFT_EVAL_NUMBERED_TEST_MESSAGE = edgeless_function_types::eval::NumberedTestMessage;

    fn handle_cast_data_in(_src: InstanceId, test_msg: Self::EFT_EVAL_NUMBERED_TEST_MESSAGE) {
        let delay = get_delay();
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

        set_delay(delay);

        log::info!("Basic Forwarder Started.");
    }

    fn handle_stop() {
        log::info!("Basic Forwarder Stopped");
    }
}

fn set_delay(delay: u64) {
    unsafe {
        FAKE_DELAY_MS = delay;
    }
}

fn get_delay() -> u64 {
    unsafe {
        FAKE_DELAY_MS
    }
}
