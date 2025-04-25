// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
use edgeless_function::*;

struct MockProducer;

edgeless_function::generate!(MockProducer);

impl MockProducerAPI<'_> for MockProducer {
    type TEST = String;

    fn handle_internal(data: &[u8]) {
        log::info!("Mock Producer Wakeup");

        let id = u32::from_ne_bytes(data.try_into().unwrap());

        cast(format!("output{}", id).as_str(), "data".as_bytes());

        delayed_cast(100, "self", &(1u32.max((id + 1) % 11)).to_ne_bytes());
    }

    fn handle_init(_payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        log::info!("Mock Producer Started. Start sending in 5s");
        delayed_cast(5000, "self", &1u32.to_ne_bytes());
    }

    fn handle_stop() {
        log::info!("Mock Producer Stopped");
    }
}
