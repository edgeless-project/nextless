DemoDashboard = edgeless_actor_class(
    id = "demo_dashboard",
    version = "0.1",
    outputs = [],
    inputs = [
        call_input("http_fetch", "edgeless.http.request", "edgeless.http.response"),
        cast_input("data_in", "eft.eval.mock_sensor_value")
    ],
    inner_structure = [
        sink("data_in"),
        sink("http_fetch")
    ],
    code = file("demo_dashboard.wasm"),
    code_type = "WASM_BASE"
)

el_main = DemoDashboard
