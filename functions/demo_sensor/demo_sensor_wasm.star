# SPDX-FileCopyrightText: © 2026 Technical University of Munich, Chair of Connected Mobility
# SPDX-License-Identifier: MIT
DemoSensor = edgeless_actor_class(
    id = "demo_sensor",
    version = "0.1",
    outputs = [
        cast_output("value", "eft.eval.mock_sensor_value")
    ],
    inputs = [],
    inner_structure = [
       source("value")
    ],
    code = file("demo_sensor.wasm"),
    code_type = "WASM_BASE"
)

el_main = DemoSensor
