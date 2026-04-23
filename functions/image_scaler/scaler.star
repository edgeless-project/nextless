# SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
# SPDX-License-Identifier: MIT
ImageScaler = edgeless_actor_class(
    id = "image_scaler",
    version = "0.1",
    outputs = [
        cast_output("scaled", "eft.vision.rawimage")
    ],
    inputs = [
        cast_input("unscaled", "eft.vision.rawimage")
    ],
    inner_structure = [
        link("unscaled", ["scaled"])
    ],
    code = file("image_scaler.tar.gz"),
    code_type = "RUST_BASE"
)

el_main = ImageScaler
