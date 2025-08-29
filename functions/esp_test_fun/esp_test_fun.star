EspTestFun = edgeless_actor_class(
    id = "esp_test_fun",
    version = "0.1",
    outputs = [cast_output("message", "String")],
    inputs = [cast_input("measurement", "String")],
    inner_structure = [link("measurement", ["message"])],
    code = file("esp_test_fun.wasm"),
    code_type = "WASM_BASE"
)

el_main = EspTestFun
