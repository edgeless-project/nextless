load("../../functions/http_requestor/http_requestor.star", "HTTPRequestor")
load("../../resources/http_egress.star", "HTTPEgress")

egress = edgeless_resource(
    id = "http-egress-1-1",
    klass = HTTPEgress,
    configurations = {}
)

requestor = edgeless_actor(
    id = "http_requestor",
    klass = HTTPRequestor,
    annotations = {}

)

requestor.http_out >> egress.new_request

wf = edgeless_workflow(
    "http_egress_example",
    [egress, requestor],
    annotations = {}
)

el_main = wf