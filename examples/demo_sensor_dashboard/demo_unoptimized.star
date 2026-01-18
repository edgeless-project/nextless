load("../../functions/demo_sensor/demo_sensor.star", "DemoSensor")
load("../../functions/demo_filter/demo_filter.star", "DemoFilter")
load("../../functions/demo_dashboard/demo_dashboard.star", "DemoDashboard")
load("../../resources/http_ingress.star", "HTTPIngress")
load("../../functions/demo_processor/demo_processor.star", "DemoProcessor")

sensor = edgeless_actor(
    id = "sensor",
    klass = DemoSensor,
    annotations = {}
)

filter = edgeless_actor(
    id = "filter",
    klass = DemoFilter,
    annotations = {},
)

dashboard = edgeless_actor(
    id = "dashboard",
    klass = DemoDashboard,
    annotations = {}
)

ingress = edgeless_resource(
    id = "demo_ingress",
    klass = HTTPIngress,
    configurations = {
        "host": "demo.localhost",
        "methods": "GET"
    }
)

reject_processor = edgeless_actor(
    id = "reject_processor",
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
reject_processor.data_in << topic("valid_measurements")

wf = edgeless_workflow(
    "sensor_dashboard_demo",
    [sensor, filter, dashboard, ingress, reject_processor],
    annotations = {
        "feature_flags": "disable_application_optimization,disable_actor_optimization",
    }
)

el_main = wf
