// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
use edgeless_function::*;

struct UnbalancedSource;

edgeless_function::generate!(UnbalancedSource);

impl UnbalancedSourceAPI<'_> for UnbalancedSource {
    type TEST = String;

    fn handle_internal(data: &[u8]) {
        log::info!("Mock Producer Wakeup");

        let id = u32::from_ne_bytes(data.try_into().unwrap());

        if id == 0 {
            cast("infrequent", "data".as_bytes());
        } else {
            cast("frequent", "data".as_bytes());
        }

        delayed_cast(100, "self", &((id + 1) % 10).to_ne_bytes());
    }

    fn handle_init(_payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        log::info!("Mock Source Started. Start sending in 5s");
        delayed_cast(5000, "self", &0u32.to_ne_bytes());
    }

    fn handle_stop() {
        log::info!("Mock Producer Stopped");
    }
}
