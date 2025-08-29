Encryptor = edgeless_actor_class(
    id = "encryptor",
    version = "0.1",
    inputs = [cast_input("data_in", "eft_eval_numbered_test_message")],
    outputs = [cast_output("data_out", "eft_eval_encrypted_numbered_test_message")],
    inner_structure = [link("data_in", ["data_out"])],
    code = file("encryptor.tar.gz"),
    code_type = "RUST_BASE"
)

el_main = Encryptor
