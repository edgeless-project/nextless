# SPDX-FileCopyrightText: © 2026 Technical University of Munich, Chair of Connected Mobility
# SPDX-License-Identifier: MIT
GameOfLifeClock = edgeless_actor_class(
    id = "game_of_life_clock",
    version = "0.1",
    outputs = [
        cast_output("trigger", "game_of_life.iteration"),
    ],
    inputs = [],
    inner_structure = [
        source("trigger"),
    ],
    # Note: During the Demo, we include a Wasm image as an extra image as compilation takes too long on a Pi 4.
    code = file("game_of_life_clock.tar.gz"),
    code_type = "RUST_BASE"
)

el_main = GameOfLifeClock
