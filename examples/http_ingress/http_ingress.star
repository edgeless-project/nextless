load("../../functions/http_processor/http_processor.star", "HTTPProcessor")
load("../../resources/http_ingress.star", "HTTPIngress")

ingress = edgeless_resource(
    id = "http-ingress-1-1",
    klass = HTTPIngress,
    configurations = {
        "host": "demo.edgeless.com",
        "methods": "POST"
    }
)

processor = edgeless_actor(
    id = "http_processor",
    klass = HTTPProcessor,
    annotations = {}

)

ingress.new_request >> processor.new_req

wf = edgeless_workflow(
    "http_ingress_example",
    [ingress, processor],
    annotations = {}
)

el_main = wf