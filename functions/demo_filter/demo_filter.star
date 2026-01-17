DemoFilter = edgeless_actor_class(
    id = "demo_filter",
    version = "0.1",
    outputs = [
        cast_output("accepted_out", "eft_eval_mock_sensor_value"),
        cast_output("rejected_out", "eft_eval_mock_sensor_value")
    ],
    inputs = [
        cast_input("data_in", "eft_eval_mock_sensor_value")
    ],
    inner_structure = [link("data_in", ["accepted_out", "rejected_out"])],
    code = file("demo_filter.tar.gz"),
    code_type = "RUST_NO_STD"
)

el_main = DemoFilter
