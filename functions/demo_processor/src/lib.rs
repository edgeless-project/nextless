// SPDX-FileCopyrightText: © 2026 Technical University of Munich, Chair of Connected Mobility
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

struct DemoProcessor;

static FAKE_DELAY_MS: std::sync::OnceLock<u64> = std::sync::OnceLock::new();

edgeless_function::generate!(DemoProcessor);

impl DemoProcessorAPI<'_> for DemoProcessor {
    type EFT_EVAL_MOCK_SENSOR_VALUE = edgeless_function_types::eval::MockSensorValue;

    fn handle_cast_data_in(_src: InstanceId, test_msg: Self::EFT_EVAL_MOCK_SENSOR_VALUE) {
        let delay = *FAKE_DELAY_MS.get().unwrap();
        log::info!(
            "Got Message: ID: {}. Value: {}. Delay: {}.",
            test_msg.sequence_number,
            test_msg.value,
            delay
        );

        if delay > 0 {
            fake_work(delay);
        }

        let mut message = test_msg.clone();

        message.value = message.value * 2.0;

        let out_message = cast_data_out(&message);
    }

    fn handle_internal(_data: &[u8]) {
        log::info!("Handle internal called.");
    }

    fn handle_init(payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        let delay = if let Some(raw_delay) = payload {
            core::str::from_utf8(raw_delay).unwrap().parse::<u64>().unwrap()
        } else {
            0
        };

        FAKE_DELAY_MS.set(delay).unwrap();

        log::info!("Actor started.");
    }

    fn handle_stop() {
        log::info!("Actor stopped.");
    }
}
