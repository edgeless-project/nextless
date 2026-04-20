PongController = edgeless_actor_class(
    id = "pong_controller",
    version = "0.1",
    outputs = [
        cast_output("render", "pong.render_request")
    ],
    inputs = [
        call_input("user_input", "eft.http.request", "eft.http.response"),
    ],
    inner_structure = [
        source("render"),
        sink("user_input"),
    ],
    code = file("pong_controller.tar.gz"),
    code_type = "RUST_BASE"
)

el_main = PongController
