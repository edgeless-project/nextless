// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
use edgeless_function::*;

struct MessageGenerator;

struct InitState {
    period: u64,
}

struct State {
    counter: u64,
}

static INIT_STATE: std::sync::OnceLock<InitState> = std::sync::OnceLock::new();
static STATE: std::sync::OnceLock<std::sync::Mutex<State>> = std::sync::OnceLock::new();

impl MessageGeneratorAPI for MessageGenerator {
    
    type STRING = String;

    
    fn handle_internal(message: &[u8]) {
        let init_state = INIT_STATE.get().unwrap();
        let mut state = STATE.get().unwrap().lock().unwrap();

        let my_id = slf();

        let msg = format!(
            "from node_id {} function_id {} [#{}]: {}",
            uuid::Uuid::from_bytes(my_id.node_id).to_string(),
            uuid::Uuid::from_bytes(my_id.component_id).to_string(),
            state.counter,
            &core::str::from_utf8(message).unwrap()
        );

        call_message(&msg);
        state.counter += 1;
        delayed_cast(init_state.period, "self", &message);
    }

    fn handle_init(payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        let arguments = if let Some(payload) = payload {
            let str_payload = core::str::from_utf8(payload).unwrap();
            edgeless_function::parse_init_payload(str_payload)
        } else {
            std::collections::HashMap::new()
        };

        let period = arguments.get("period").unwrap_or(&"1000").parse::<u64>().unwrap_or(1000);
        let message = arguments.get("message").unwrap_or(&"hello world");
        let _ = INIT_STATE.set(InitState { period });

        let _ = STATE.set(std::sync::Mutex::new(State { counter: 0 }));

        cast("self", message.as_bytes());
    }

    fn handle_stop() {
        // noop
    }
}

edgeless_function::generate!(MessageGenerator);
