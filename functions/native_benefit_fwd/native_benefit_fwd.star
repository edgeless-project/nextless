NativeBenefitFwd = edgeless_actor_class(
    id = "native_benefit_fwd",
    version = "0.1",
    outputs = [cast_output("data_out", "eft.eval.numbered_test_message")],
    inputs = [cast_input("data_in", "eft.eval.numbered_test_message")],
    inner_structure = [link("data_in", ["data_out"])],
    code = file("native_benefit_fwd.tar.gz"),
    code_type = "RUST_NO_STD"
)

el_main = NativeBenefitFwd
