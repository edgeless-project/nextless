HTTPIngress = edgeless_resource_class(
    id = "http-ingress",
    outputs = [call_output("new_request", "edgeless.http.request", "edgeless.http.response")],
    inputs = [],
    inner_structure = [source("new_request")],
)

el_main = HTTPIngress
