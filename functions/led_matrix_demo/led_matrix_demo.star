LedMatrixDemo = edgeless_actor_class(
    id = "led_matrix_demo",
    version = "0.1",
    outputs = [
        cast_output("value", "eft.led_matrix.matrix_frame")
    ],
    inputs = [],
    inner_structure = [
       source("value")
    ],
    code = file("led_matrix_demo.wasm"),
    code_type = "WASM_BASE"
)

el_main = LedMatrixDemo
