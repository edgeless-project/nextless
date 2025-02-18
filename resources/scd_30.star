SCD30Sensor = edgeless_resource_class(
    id = "scd30-sensor",
    outputs = [cast_output("data_out", "String")],
    inputs = [],
    inner_structure = [source("data_out")],
)

el_main = SCD30Sensor
