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
        cast_output("drawable", "eft.led_matrix.matrix_frame"),
        cast_output("iteration_clock_o", "game_of_life.iteration"),
    ],
    inputs = [
        cast_input("update_left_i", "game_of_life.update_col"),
        cast_input("update_right_i", "game_of_life.update_col"),
        cast_input("update_top_i", "game_of_life.update_row"),
        cast_input("update_bottom_i", "game_of_life.update_row"),
        cast_output("update_top_left_i", "game_of_life.update_corner"),
        cast_output("update_top_right_i", "game_of_life.update_corner"),
        cast_output("update_bottom_left_i", "game_of_life.update_corner"),
        cast_output("update_bottom_right_i", "game_of_life.update_corner"),
        cast_input("iteration_clock_i", "game_of_life.iteration"),
    ],
    inner_structure = [
       source("update_left_o"),
       source("update_right_o"),
       source("update_top_o"),
       source("update_bottom_o"),
       source("update_top_left_o"),
       source("update_top_right_o"),
       source("update_bottom_left_o"),
       source("update_bottom_right_o"),
       source("drawable"),
       source("iteration_clock_o"),
       sink("update_left_i"),
       sink("update_right_i"),
       sink("update_top_i"),
       sink("update_bottom_i"),
       sink("update_top_left_i"),
       sink("update_top_right_i"),
       sink("update_bottom_left_i"),
       sink("update_bottom_right_i"),
       sink("iteration_clock_i")

    ],
    code = file("game_of_life.wasm"),
    code_type = "WASM_BASE"
)

el_main = GameOfLife
