DemoSensor = edgeless_actor_class(
    id = "demo_sensor",
    version = "0.1",
    outputs = [
        cast_output("value", "eft_eval_mock_sensor_value")
    ],
    inputs = [],
    inner_structure = [
       source("value")
    ],
    code = file("demo_sensor.tar.gz"),
    code_type = "RUST_BASE"
)

el_main = DemoSensor
