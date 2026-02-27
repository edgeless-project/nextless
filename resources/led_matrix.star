LedMatrix = edgeless_resource_class(
    id = "led-matrix",
    outputs = [],
    inputs = [cast_input("update", "eft.led_matrix.matrix_frame")],
    inner_structure = [sink("update")],
)

el_main = LedMatrix
