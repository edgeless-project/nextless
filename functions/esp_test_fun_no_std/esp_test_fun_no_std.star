EspSensorP = edgeless_actor_class(
    id = "esp_sensor_p",
    version = "0.1",
    outputs = [cast_output("message", "String")],
    inputs = [cast_input("measurement", "String")],
    inner_structure = [link("measurement", ["message"])],
    code = file("esp_sensor_p.tar.gz"),
    code_type = "RUST_NO_STD"
)

el_main = EspSensorP
