DetectionTester = edgeless_actor_class(
    id = "detection_tester",
    version = "0.1",
    outputs = [cast_output("test_image", "eft_vision_rawimage")],
    inputs = [cast_input("detection", "String")],
    inner_structure = [source("test_image"), sink("detection")],
    code = file("detection_tester.tar.gz"),
    code_type = "RUST_BASE"
)

el_main = DetectionTester
