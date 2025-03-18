// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
#![no_std]

use edgeless_function::*;

struct TestFun;

edgeless_function::generate!(TestFun);

impl<'c> EspTestFunAPI<'c> for TestFun {
    type STRING = &'c str;

    fn handle_cast_measurement(_src: InstanceId, str_message: &'c str) {
        // let str_message = core::str::from_utf8(encoded_message).unwrap();
        log::info!("Resource Processor: 'Cast' called, MSG: {}", str_message);
        let mut values = str_message.split(";");
        if let Some(co2) = values.next() {
            let co2: f32 = co2.parse().unwrap();
            let msg = if co2 > 800.0 { "high" } else { "low" };
            cast_message(&msg);
        }
    }

    fn handle_internal(_data: &[u8]) {
        log::info!("Resource Processor: 'Internal' called");
    }

    fn handle_init(_payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        log::info!("Resource Processor: 'Init' called");
    }

    fn handle_stop() {
        log::info!("Resource Processor: 'Stop' called");
    }
}
