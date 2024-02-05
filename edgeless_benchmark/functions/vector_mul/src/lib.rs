// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use edgeless_function::api::*;
use std::num::Wrapping;

struct VectorMulFunction;

// Parameters from glib's implementation.
const MODULUS: Wrapping<u32> = Wrapping(2147483648);
const MULTIPLIER: Wrapping<u32> = Wrapping(1103515245);
const OFFSET: Wrapping<u32> = Wrapping(12345);

struct Lcg {
    seed: Wrapping<u32>,
}

impl Lcg {
    fn new(seed: u32) -> Self {
        Self { seed: Wrapping(seed) }
    }

    fn rand(&mut self) -> f32 {
        self.seed = (MULTIPLIER * self.seed + OFFSET) % MODULUS;
        self.seed.0 as f32 / MODULUS.0 as f32
    }
}

fn make_new_matrix(lcg: &mut Lcg, size: usize) -> Vec<f32> {
    let mut new_matrix = vec![0.0; size * size];
    for value in new_matrix.iter_mut() {
        *value = lcg.rand();
    }
    new_matrix
}

fn make_new_vector(lcg: &mut Lcg, size: usize) -> Vec<f32> {
    let mut new_vector = vec![0.0; size];
    for value in new_vector.iter_mut() {
        *value = lcg.rand();
    }
    new_vector
}

struct Conf {
    // True: this is the client, which triggers the first input and receives the last output.
    is_client: bool,
    // Name of the workflow (for stats only).
    wf_name: String,
    // Name of the function (for stats only).
    fun_name: String,
    // Input size of the vector.
    input_size: usize,
}
struct State {
    // ID of the next transaction. Only used if is_client == true.
    next_id: usize,
    // Pseudo-random number generator.
    lcg: Lcg,
    // Matrix of values to consume CPU in processing functions. Unused by clients.
    matrix: Vec<f32>,
}

static CONF: std::sync::OnceLock<Conf> = std::sync::OnceLock::new();
static STATE: std::sync::OnceLock<std::sync::Mutex<State>> = std::sync::OnceLock::new();

fn parse_init(payload: &str) -> std::collections::HashMap<&str, &str> {
    let tokens = payload.split(',');
    let mut arguments = std::collections::HashMap::new();
    for token in tokens {
        let mut inner_tokens = token.split('=');
        if let Some(key) = inner_tokens.next() {
            if let Some(value) = inner_tokens.next() {
                arguments.insert(key, value);
            } else {
                log::error!("invalid initialization token: {}", token);
            }
        } else {
            log::error!("invalid initialization token: {}", token);
        }
    }
    arguments
}

impl Edgefunction for VectorMulFunction {
    fn handle_cast(_src: InstanceId, encoded_message: String) {
        let conf = CONF.get().unwrap();
        // log::info!("VectorMul casted, wf {}, fun {}, MSG: {}", conf.wf_name, conf.fun_name, encoded_message);
        let mut state = STATE.get().unwrap().lock().unwrap();

        //
        // Client
        //
        if conf.is_client {
            let id = state.next_id;
            if id > 0 {
                cast("metric", format!("workflow:end:{}:{}", conf.wf_name, id).as_str());
            }

            state.next_id += 1;
            let random_input = make_new_vector(&mut state.lcg, conf.input_size);
            let payload = format!(
                "{},{}",
                state.next_id,
                random_input.iter().map(|x| format!("{}", x)).collect::<Vec<String>>().join(",")
            );

            cast("metric", format!("workflow:start:{}:{}", conf.wf_name, state.next_id).as_str());
            cast("out", &payload);

        //
        // Processing function
        //
        } else {
            let input = encoded_message.split(',').map(|x| x.parse::<f32>().unwrap_or(0.0)).collect::<Vec<f32>>();
            let n = conf.input_size;
            assert!(input.len() == (1 + n));
            let id = input[0] as usize;
            cast("metric", format!("function:start:{}:{}:{}", conf.wf_name, conf.fun_name, id).as_str());

            // Produce the output by multiplying the internal matrix by the input.
            let mut output = vec![0.0_f32; n];
            for i in 0..n {
                for j in 0..n {
                    output[i] += state.matrix[i * n + j] * input[j];
                }
            }
            cast(
                "out",
                format!("{},{}", id, output.iter().map(|x| format!("{}", x)).collect::<Vec<String>>().join(",")).as_str(),
            );
            cast("metric", format!("function:end:{}:{}:{}", conf.wf_name, conf.fun_name, id).as_str());
        }
    }

    fn handle_call(_src: InstanceId, _encoded_message: String) -> CallRet {
        log::info!("VectorMul called: ignored");
        CallRet::Noreply
    }

    // example of payload:
    // seed=42,is_client=true,is_last=false,wf_name=my_workflow,fun_name=my_function,input_size=1000
    fn handle_init(payload: String, _serialized_state: Option<String>) {
        edgeless_function::init_logger();
        log::info!("VectorMul initialized, payload: {}", payload);
        let arguments = parse_init(&payload);

        let seed = arguments.get("seed").unwrap_or(&"0").parse::<u32>().unwrap_or(0);

        let is_client = arguments.get("is_client").unwrap_or(&"false").to_lowercase() == "true";
        let wf_name = arguments.get("wf_name").unwrap_or(&"no-wf-name").to_string();
        if wf_name == "no-wf-name" {
            log::warn!("workflow name not specified, using: no-wf-name");
        }
        let fun_name = arguments.get("fun_name").unwrap_or(&"no-fun-name").to_string();
        if fun_name == "no-fun-name" {
            log::warn!("workflow name not specified, using: no-fun-name");
        }
        let input_size = arguments.get("input_size").unwrap_or(&"100").parse::<usize>().unwrap_or(100);

        let _ = CONF.set(Conf {
            is_client,
            wf_name,
            fun_name,
            input_size,
        });

        let mut lcg = Lcg::new(seed);
        let matrix = make_new_matrix(
            &mut lcg,
            match is_client {
                true => 0,
                false => input_size,
            },
        );

        let _ = STATE.set(std::sync::Mutex::new(State { next_id: 0, lcg, matrix }));

        if is_client {
            delayed_cast(1000, "self", "");
        }
    }

    fn handle_stop() {
        let conf = CONF.get().unwrap();
        log::info!("VectorMul stopped, wf {}, fun {}", conf.wf_name, conf.fun_name);
    }
}

edgeless_function::export!(VectorMulFunction);