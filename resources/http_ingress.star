HTTPIngress = edgeless_resource_class(
    id = "http-ingress",
    outputs = [call_output("new_request", "eft.http.request", "eft.http.response")],
    inputs = [],
    inner_structure = [source("new_request")],
)

el_main = HTTPIngress
