FileLog = edgeless_resource_class(
    id = "file-log",
    outputs = [],
    inputs = [call_input("line", "String")],
    inner_structure = [sink("line")],   
)

el_main = FileLog