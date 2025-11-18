EspTestFunNoStd = edgeless_actor_class(
    id = "esp_test_fun_no_std",
    version = "0.1",
    outputs = [cast_output("message", "String")],
    inputs = [cast_input("measurement", "String")],
    inner_structure = [link("measurement", ["message"])],
    code = file("esp_test_fun_no_std.wasm"),
    code_type = "RUST_NO_STD"
)

el_main = EspTestFunNoStd
