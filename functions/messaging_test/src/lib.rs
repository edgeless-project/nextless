// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
use edgeless_function::*;
use log;

struct MessagingTest;

edgeless_function::generate!(MessagingTest);

impl MessagingTestAPI for MessagingTest {
    type STRING = String;
    
    fn handle_cast_test_cast_input(src: InstanceId, message: String) {
        match message.as_str() {
            "test_cast_raw_output" => {
                cast_raw(src, "test", "cast_raw_output".as_bytes());
            }
            "test_call_raw_output" => {
                let _res = call_raw(src, "test", "call_raw_output".as_bytes());
            }
            "test_delayed_cast_output" => {
                delayed_cast(100, "test_cast", "delayed_cast_output".as_bytes());
            }
            "test_cast_output" => {
                cast_test_cast(&"cast_output".to_string());
            }
            "test_call_output" => {
                let _res = call_test_call(&"call_output".to_string());
            }
            _ => {
                log::info!("Unprocessed Message");
            }
        }
    }

    fn handle_call_test_input_reply(_src: InstanceId, _message: String) -> String {
        "test_reply".to_string()
    }

    fn handle_call_test_input_noreply(_src: InstanceId, _message: String){
    }

    fn handle_internal(_message: &[u8]) {}

    fn handle_init(_payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        log::error!("Messaging Test Init");
    }

    fn handle_stop() {
        log::info!("Messaging Test Stop");
    }
}
