YoloxNano = edgeless_actor_class(
    id = "yolox_nano",
    version = "0.1",
    outputs = [cast_output("detection", "String"), cast_output("annotated_image", "eft.vision.rawimage")],
    inputs = [cast_input("image", "eft.vision.rawimage")],
    inner_structure = [link("image", ["detection", "annotated_image"])],
    code = file("yolox_nano.tar.gz"),
    code_type = "RUST_WGPU"
)

el_main = YoloxNano
