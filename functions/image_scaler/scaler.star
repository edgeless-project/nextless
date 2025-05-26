ImageScaler = edgeless_actor_class(
    id = "image_scaler",
    version = "0.1",
    outputs = [
        cast_output("scaled", "eft_vision_rawimage")
    ],
    inputs = [
        cast_input("unscaled", "eft_vision_rawimage")
    ],
    inner_structure = [
        link("unscaled", ["scaled"])
    ],
    code = file("image_scaler.wasm"),
    code_type = "RUST_WASM"
)

el_main = ImageScaler
