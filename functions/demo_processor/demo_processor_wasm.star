DemoProcessor = edgeless_actor_class(
    id = "demo_processor",
    version = "0.1",
    outputs = [cast_output("data_out", "eft.eval.mock_sensor_value")],
    inputs = [cast_input("data_in", "eft.eval.mock_sensor_value")],
    inner_structure = [link("data_in", ["data_out"])],
    code = file("demo_processor.wasm"),
    code_type = "WASM_BASE"
)

el_main = DemoProcessor
