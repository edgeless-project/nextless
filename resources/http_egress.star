HTTPEgress = edgeless_resource_class(
    id = "http-egress",
    outputs = [],
    inputs = [call_input("new_request", "edgeless.http.request", "edgeless.http.response")],
    inner_structure = [sink("new_request")],
)

el_main = HTTPEgress
