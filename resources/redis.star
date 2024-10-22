Redis = edgeless_resource_class(
    id = "redis",
    outputs = [],
    inputs = [cast_input("line", "String")],
    inner_structure = [sink("line")],   
)

el_main = Redis