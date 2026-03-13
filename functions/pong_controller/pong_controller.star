PongController = edgeless_actor_class(
    id = "pong_controller",
    version = "0.1",
    outputs = [
        cast_output("render", "pong.render_request")
    ],
    inputs = [
        call_input("user_input", "edgeless.http.request", "edgeless.http.response"),
    ],
    inner_structure = [
        source("render"),
        sink("user_input"),
    ],
    code = file("pong_controller.wasm"),
    code_type = "WASM_BASE"
)

el_main = PongController
