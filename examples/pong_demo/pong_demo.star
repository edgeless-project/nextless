load("../../resources/http_ingress.star", "HTTPIngress")
load("../../resources/led_matrix.star", "LedMatrix")
load("../../functions/pong_controller/pong_controller.star", "PongController")
load("../../functions/pong_renderer/pong_renderer.star", "PongRenderer")
load("two_x_three.star", "instances")
load("addr_conf.star", "host")

def id_str(id):
    unpadded = "%x" % (id)
    padded = ""
    for i in range(12 - len(unpadded)):
        padded = padded + "0"
    padded = padded + unpadded

    full = "00000000-0000-0000-0000-{}".format(padded)
    return full

controller = edgeless_actor(
    id = "controller",
    klass = PongController,
    annotations = {
        "node_ids_allowed": id_str(1),
        "init-payload": "size_y={},size_x={}".format(128, 192)
    }
)

renderers = {
    instance["id"]: edgeless_actor(
    id = "renderer_{}".format(instance["id"]),
    klass = PongRenderer,
    annotations = {
        "node_ids_allowed": id_str(instance["id"]),
        "init-payload": "position_y={},position_x={}".format(instance["position_y"]*64, instance["position_x"]*64)
    }
) for instance in instances}

displays = {
    instance["id"]: edgeless_resource(
    id = "led-matrix-1-{}".format(instance["id"]),
    klass = LedMatrix,
    configurations = {},
    annotations = {
        "node_ids_allowed": id_str(instance["id"]),
    }
) for instance in instances}

ingress = edgeless_resource(
    id = "controller_ingress",
    klass = HTTPIngress,
    configurations = {
        "host": host,
        "methods": "POST,GET"
    }
)

ingress.new_request >> controller.user_input
[renderers[instance["id"]].drawable >> displays[instance["id"]].update for instance in instances]
controller.render >> [renderer.render for renderer in renderers.values()]

wf = edgeless_workflow(
    "pong_demo",
    [controller, ingress] + renderers.values() + displays.values(),
    annotations = {
        "feature_flags": "disable_ip_multicast_dialect",
    }
)

el_main = wf
