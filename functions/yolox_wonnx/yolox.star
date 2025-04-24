YoloxNano = edgeless_actor_class(
    id = "yolox_nano",
    version = "0.1",
    outputs = [cast_output("detection", "String"), cast_output("annotated_image", "eft_vision_rawimage")],
    inputs = [cast_input("image", "eft_vision_rawimage")],
    inner_structure = [link("image", ["detection", "annotated_image"])],
    code = file("yolox_nano.wasm"),
    code_type = "RUST_WASM"
)

el_main = YoloxNano
