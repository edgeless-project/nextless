# SPDX-FileCopyrightText: © 2026 Technical University of Munich, Chair of Connected Mobility
# SPDX-License-Identifier: MIT
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
    # Note: During the Demo, we include a Wasm image as an extra image as compilation takes too long on a Pi 4.
    code = file("pong_controller.tar.gz"),
    code_type = "RUST_BASE"
)

el_main = PongController
