GPUExample = edgeless_actor_class(
    id = "gpu_example",
    version = "0.1",
    outputs = [],
    inputs = [cast_input("trigger", "String")],
    inner_structure = [sink("trigger")],
    code = file("gpu_example.wasm"),
    code_type = "WASM_WGPU"
)

el_main = GPUExample
