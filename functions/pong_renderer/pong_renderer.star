PongRenderer = edgeless_actor_class(
    id = "pong_renderer",
    version = "0.1",
    outputs = [
        cast_output("drawable", "eft.led_matrix.matrix_frame")
    ],
    inputs = [
        cast_input("render", "pong.render_request"),
    ],
    inner_structure = [
        link("render", ["drawable"])
    ],
    # Note: During the Demo, we include a Wasm image as an extra image as compilation takes too long on a Pi 4.
    code = file("pong_renderer.tar.gz"),
    code_type = "RUST_BASE"
)

el_main = PongRenderer
