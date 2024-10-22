HTTPEgress = edgeless_resource_class(
    id = "http-egress",
    outputs = [],
    inputs = [call_input("new_request", "edgeless.http.Request", "edgeless.http.Response")],
    inner_structure = [sink("new_request")],   
)

el_main = HTTPEgress