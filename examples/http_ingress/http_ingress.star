# SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
# SPDX-License-Identifier: MIT
load("../../functions/http_processor/http_processor.star", "HTTPProcessor")
load("../../resources/http_ingress.star", "HTTPIngress")
load("../../resources/file_log.star", "FileLog")

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
    annotations = {
        "min_instances": "10",
        "max_instances": "10",
    }

)


logger = edgeless_resource(
    id = "my-log",
    klass = FileLog,
    configurations = {
        "filename": "my-local-file.log",
        "add-timestamp": "true"
    }
)


ingress.new_request >> any([processor.new_req])
processor.log_value >>  logger.line

wf = edgeless_workflow(
    "http_ingress_example",
    [ingress, processor, logger],
    annotations = {}
)

el_main = wf
