EPaperDisplay = edgeless_resource_class(
    id = "epaper-display",
    outputs = [],
    inputs = [cast_input("text", "String")],
    inner_structure = [sink("text")],
)

el_main = EPaperDisplay
