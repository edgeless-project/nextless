GameOfLife = edgeless_actor_class(
    id = "game_of_life",
    version = "0.1",
    outputs = [
        cast_output("update_left_o", "game_of_life.update_col"),
        cast_output("update_right_o", "game_of_life.update_col"),
        cast_output("update_top_o", "game_of_life.update_row"),
        cast_output("update_bottom_o", "game_of_life.update_row"),
        cast_output("update_top_left_o", "game_of_life.update_corner"),
        cast_output("update_top_right_o", "game_of_life.update_corner"),
        cast_output("update_bottom_left_o", "game_of_life.update_corner"),
        cast_output("update_bottom_right_o", "game_of_life.update_corner"),
        cast_output("drawable", "eft.led_matrix.matrix_frame", optional=False),
    ],
    inputs = [
        cast_input("update_left_i", "game_of_life.update_col"),
        cast_input("update_right_i", "game_of_life.update_col"),
        cast_input("update_top_i", "game_of_life.update_row"),
        cast_input("update_bottom_i", "game_of_life.update_row"),
        cast_input("update_top_left_i", "game_of_life.update_corner"),
        cast_input("update_top_right_i", "game_of_life.update_corner"),
        cast_input("update_bottom_left_i", "game_of_life.update_corner"),
        cast_input("update_bottom_right_i", "game_of_life.update_corner"),
        cast_input("iteration_clock_i", "game_of_life.iteration", optional=False),
    ],
    inner_structure = [
       link("iteration_clock_i", [
           "update_left_o",
           "update_right_o",
           "update_top_o",
           "update_bottom_o",
           "update_top_left_o",
           "update_top_right_o",
           "update_bottom_left_o",
           "update_bottom_right_o",
           "drawable"
       ]),
       link("update_left_i", ["drawable"]),
       link("update_right_i", ["drawable"]),
       link("update_top_i", ["drawable"]),
       link("update_bottom_i", ["drawable"]),
       link("update_top_left_i", ["drawable"]),
       link("update_top_right_i", ["drawable"]),
       link("update_bottom_left_i", ["drawable"]),
       link("update_bottom_right_i", ["drawable"]),

    ],
    code = file("game_of_life.wasm"),
    code_type = "WASM_BASE"
)

el_main = GameOfLife
