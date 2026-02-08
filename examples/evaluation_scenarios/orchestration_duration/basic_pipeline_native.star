load("../../../functions/demo_sensor/demo_sensor_wasm.star", "DemoSensor")
load("../../../functions/demo_dashboard/demo_dashboard_wasm.star", "DemoDashboard")
load("../../../resources/http_ingress.star", "HTTPIngress")
load("../../../functions/native_processor/native_processor.star", "NativeProcessor")
load("./configuration.star", "PROCESSOR_INIT_NODE", "SENSOR_ALLOWED_NODES", "DASHBOARD_ALLOWED_NODES", "PROCESSOR_ALLOWED_NODES")

sensor = edgeless_actor(
    id = "sensor",
    klass = DemoSensor,
    annotations = {
        "scaling_mode": "all_nodes",
        # Node labels currently lead to broken scaling behavior.
        # "node_label_filter_allowed": "sensor",
        "node_ids_allowed": SENSOR_ALLOWED_NODES,
    }
)

processor = edgeless_actor(
    id = "processor",
    klass = NativeProcessor,
    annotations = {
        # Mock Delay
        "init-payload": "100",
        # Node labels currently lead to broken scaling behavior.
        # "node_label_filter_allowed": "processor",
        "node_ids_allowed": PROCESSOR_ALLOWED_NODES,
        "node_id_init_on": PROCESSOR_INIT_NODE
    }
)

dashboard = edgeless_actor(
    id = "dashboard",
    klass = DemoDashboard,
    annotations = {
        # Node labels currently lead to broken scaling behavior.
        # "node_label_filter_allowed": "dashboard",
        "node_ids_allowed": DASHBOARD_ALLOWED_NODES,
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
        # Currently not used (scaling resources does not work correctly).
        # This is instead enforced by only having one node with in ingress.
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
