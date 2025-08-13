// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
use edgeless_function::*;

extern "C" {
    fn eval_timestamp_ns() -> u64;
}

fn get_timestamp() -> u64 {
    unsafe { eval_timestamp_ns() }
}

#[derive(Debug)]
struct Configuration {
    iter_message_delay_ms: u64,
    payload_size: u64,
    num_iterations: u64,
}

static STATE: std::sync::OnceLock<std::sync::Mutex<HarnessState>> = std::sync::OnceLock::new();
static CONFIGURATION: std::sync::OnceLock<Configuration> = std::sync::OnceLock::new();

#[derive(Debug)]
struct HarnessState {
    next_iteration_id: u64,
    start_times: std::collections::HashMap<u64, u64>,
}

struct LatencyHarness;

edgeless_function::generate!(LatencyHarness);

impl LatencyHarnessAPI<'_> for LatencyHarness {
    type EFT_EVAL_NUMBERED_TEST_MESSAGE = edgeless_function_types::eval::NumberedTestMessage;

    fn handle_internal(_: &[u8]) {
        let mut state = STATE.get().unwrap().lock().unwrap();
        let configuration = CONFIGURATION.get().unwrap();
        let payload_size = configuration.payload_size;

        let iteration_id = state.next_iteration_id;
        state.next_iteration_id += 1;

        log::info!("Latency Harness produce. Iteration: {iteration_id}. Payload size: {payload_size}");

        let payload = String::from_utf8(vec!['a' as u8; payload_size as usize]).unwrap();
        let start = get_timestamp();
        cast_start(&edgeless_function_types::eval::NumberedTestMessage {
            sequence_number: iteration_id,
            payload: payload,
        });
        state.start_times.insert(iteration_id, start);

        if configuration.num_iterations == 0 || state.next_iteration_id < configuration.num_iterations {
            delayed_cast(configuration.iter_message_delay_ms, "self", &[]);
        }
    }

    fn handle_cast_end(_src: InstanceId, payload: Self::EFT_EVAL_NUMBERED_TEST_MESSAGE) {
        let end = get_timestamp();

        let mut state = STATE.get().unwrap().lock().unwrap();

        let id = payload.sequence_number;

        let start_time = state.start_times.remove(&id);

        if let Some(start) = start_time {
            let iter_dur = std::time::Duration::from_nanos(end - start);
            log::info!("Latency Harness End. Iteration: {id}. Latency: {iter_dur:?}");
        } else {
            log::info!("Latency Harness End. Iteration: {id}. Unknown Latency.");
        }
    }

    fn handle_init(payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        let (delay_ms, payload_size, num_iterations) = if let Some(payload) = payload {
            let payload_str = String::from_utf8(payload.to_vec()).unwrap();
            let configuration: Vec<_> = payload_str.split(",").collect();
            assert!(configuration.len() == 3);
            (
                configuration[0].parse::<u64>().unwrap(),
                configuration[1].parse::<u64>().unwrap(),
                configuration[1].parse::<u64>().unwrap(),
            )
        } else {
            (100, 1000, 0)
        };
        CONFIGURATION
            .set(Configuration {
                iter_message_delay_ms: delay_ms,
                payload_size: payload_size,
                num_iterations,
            })
            .unwrap();
        STATE
            .set(std::sync::Mutex::new(HarnessState {
                next_iteration_id: 0,
                start_times: std::collections::HashMap::new(),
            }))
            .unwrap();
        log::info!("Latency Harness started. Start sending in 5s. Delay between messages: {delay_ms}. Payload size: {payload_size}");
        delayed_cast(5000, "self", &[]);
    }

    fn handle_stop() {
        log::info!("Latency Harness stopped.");
    }
}
