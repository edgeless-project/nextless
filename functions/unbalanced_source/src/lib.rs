// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
use edgeless_function::*;

struct UnbalancedSource;

edgeless_function::generate!(UnbalancedSource);

static COUNTER: std::sync::OnceLock<std::sync::Mutex<u64>> = std::sync::OnceLock::new();

impl UnbalancedSourceAPI<'_> for UnbalancedSource {
    type EFT_EVAL_NUMBERED_TEST_MESSAGE = edgeless_function_types::eval::NumberedTestMessage;

    fn handle_internal(data: &[u8]) {
        log::info!("Mock Producer Wakeup");

        let mut state = COUNTER.get().unwrap().lock().unwrap();
        let iteration_id = *state;
        *state += 1;

        let id = u32::from_ne_bytes(data.try_into().unwrap());

        let payload = edgeless_function_types::eval::NumberedTestMessage {
            sequence_number: iteration_id,
            payload: "data".to_string(),
        };

        if id == 0 {
            cast_infrequent(&payload);
        } else {
            cast_frequent(&payload);
        }

        delayed_cast(100, "self", &((id + 1) % 10).to_ne_bytes());
    }

    fn handle_init(_payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        log::info!("Mock Source Started. Start sending in 5s");
        COUNTER.set(std::sync::Mutex::new(0)).unwrap();
        delayed_cast(5000, "self", &0u32.to_ne_bytes());
    }

    fn handle_stop() {
        log::info!("Mock Producer Stopped");
    }
}
