# SPDX-FileCopyrightText: © 2026 Technical University of Munich, Chair of Connected Mobility
# SPDX-License-Identifier: MIT
load("../../functions/demo_sensor/demo_sensor.star", "DemoSensor")
load("../../functions/demo_filter/demo_filter.star", "DemoFilter")
load("../../functions/demo_dashboard/demo_dashboard.star", "DemoDashboard")
load("../../resources/http_ingress.star", "HTTPIngress")
load("../../functions/demo_processor/demo_processor.star", "DemoProcessor")

sensor = edgeless_actor(
    id = "sensor",
    klass = DemoSensor,
    annotations = {
        "scaling_mode": "all_nodes"
    }
)

filter = edgeless_actor(
    id = "filter",
    klass = DemoFilter,
    annotations = {
        "scaling_mode": "all_nodes"
    },
)

dashboard = edgeless_actor(
    id = "dashboard",
    klass = DemoDashboard,
    annotations = {
        # "scaling_mode": "scalable"
    }
)

ingress = edgeless_resource(
    id = "demo_ingress",
    klass = HTTPIngress,
    configurations = {
        "host": "demo.localhost",
        "methods": "GET"
    },
    annotations = {
        "scaling_mode": "all_nodes"
    },
)

processor = edgeless_actor(
    id = "processor",
    klass = DemoProcessor,
    annotations = {
        # Mock Delay
        "init-payload": "100"
    }
)

sensor.value >> filter.data_in
ingress.new_request >> dashboard.http_fetch
filter.accepted_out >> topic("valid_measurements")
dashboard.data_in << topic("valid_measurements")
# Useless Interaction
processor.data_in << topic("valid_measurements")

wf = edgeless_workflow(
    "sensor_dashboard_demo",
    [sensor, filter, dashboard, ingress, processor],
    annotations = {}
)

el_main = wf
