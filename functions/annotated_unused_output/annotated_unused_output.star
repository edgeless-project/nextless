AnnotatedUnusedOutput = edgeless_actor_class(
    id = "annotated_unused_output",
    version = "0.1",
    outputs = [cast_output("data_out", "eft_eval_numbered_test_message"), cast_output("unused_out", "eft_eval_numbered_test_message")],
    inputs = [cast_input("data_in", "eft_eval_numbered_test_message")],
    inner_structure = [link("data_in", ["data_out", "unused_out"])],
    code = file("annotated_unused_output.tar.gz"),
    code_type = "RUST_BASE"
)

el_main = AnnotatedUnusedOutput
