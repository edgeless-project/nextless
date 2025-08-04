// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
use edgeless_function::*;

extern "C" {
    fn eval_timestamp_ns() -> u64;
}

fn get_timestamp() -> u64 {
    unsafe { eval_timestamp_ns() }
}

static STATE: std::sync::OnceLock<std::sync::Mutex<HarnessState>> = std::sync::OnceLock::new();

#[derive(Debug)]
struct HarnessState {
    next_iteration_id: u32,
    start_times: std::collections::HashMap<u32, u64>,
}

struct LatencyHarness;

edgeless_function::generate!(LatencyHarness);

impl LatencyHarnessAPI<'_> for LatencyHarness {
    type TEST_ID = String;

    fn handle_internal(delay_raw: &[u8]) {
        let mut state = STATE.get().unwrap().lock().unwrap();
        let iteration_id = state.next_iteration_id;
        state.next_iteration_id += 1;

        log::info!("Latency Harness Produce. Iteration: {iteration_id}");

        let delay = u64::from_ne_bytes(delay_raw.try_into().unwrap());

        let data = format!("{iteration_id}");
        let start = get_timestamp();
        cast_start(&data);
        state.start_times.insert(iteration_id, start);

        delayed_cast(delay, "self", delay_raw);
    }

    fn handle_cast_end(_src: InstanceId, id_raw: Self::TEST_ID) {
        let end = get_timestamp();

        let mut state = STATE.get().unwrap().lock().unwrap();

        let id = id_raw.parse::<u32>().unwrap();

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
        let delay_ms = if let Some(payload) = payload {
            core::str::from_utf8(payload).unwrap().parse::<u64>().unwrap()
        } else {
            100
        };
        STATE
            .set(std::sync::Mutex::new(HarnessState {
                next_iteration_id: 0,
                start_times: std::collections::HashMap::new(),
            }))
            .unwrap();
        log::info!("Latency Harness Started. Start sending in 5s. Delay Between Messages: {delay_ms}.");
        delayed_cast(5000, "self", &delay_ms.to_ne_bytes());
    }

    fn handle_stop() {
        log::info!("Latency Harness Stopped.");
    }
}
