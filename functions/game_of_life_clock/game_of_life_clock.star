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
    code = file("game_of_life_clock.wasm"),
    code_type = "WASM_BASE"
)

el_main = GameOfLifeClock
