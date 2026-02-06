load("../../../functions/demo_sensor/demo_sensor.star", "DemoSensor")
load("../../../functions/demo_filter/demo_filter.star", "DemoFilter")
load("../../../functions/demo_dashboard/demo_dashboard.star", "DemoDashboard")
load("../../../resources/http_ingress.star", "HTTPIngress")
load("../../../functions/demo_processor/demo_processor.star", "DemoProcessor")

sensor = edgeless_actor(
    id = "sensor",
    klass = DemoSensor,
    annotations = {
        "scaling_mode": "all_nodes",
        "node_label_filter_allowed": "sensor",
    }
)

processor = edgeless_actor(
    id = "processor",
    klass = DemoProcessor,
    annotations = {
        # Mock Delay
        "init-payload": "100",
        "node_label_filter_allowed": "processor",
        "node_id_init_on": "00000000-0000-0000-0000-000000000002"
    }
)

dashboard = edgeless_actor(
    id = "dashboard",
    klass = DemoDashboard,
    annotations = {
        "node_label_filter_allowed": "dashboard",
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
        # currently not used (scaling resources does not work correctly), instead enforced by only having one node with in ingress.
        # "scaling_mode": "all_nodes",
        # "node_label_filter_allowed": "dashboard",
    },
)

sensor.value >> processor.data_in
processor.data_out >> dashboard.data_in
ingress.new_request >> dashboard.http_fetch

wf = edgeless_workflow(
    "orchestration_duration",
    [sensor, dashboard, ingress, processor],
    annotations = {}
)

el_main = wf
