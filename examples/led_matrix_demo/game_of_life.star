load("../../functions/game_of_life/game_of_life.star", "GameOfLife")
load("../../resources/led_matrix.star", "LedMatrix")
load("three_x_three.star", "instances")

def id_str(id):
    unpadded = "%x" % (id)
    padded = ""
    for i in range(12 - len(unpadded)):
        padded = padded + "0"
    padded = padded + unpadded

    full = "00000000-0000-0000-0000-{}".format(padded)
    return full

def init_payload(instance):
    base = "draw_border=true,corner_blocks=false,position_y={},position_x={}".format(instance["position_y"],instance["position_x"])
    if instance["id"] == 1:
        return "period_ms=500," + base
    if instance["id"] == 3:
        return "period_ms=0,periodic_glider=true," + base
    return "period_ms=0," + base

game_instances = {
    instance["id"]: edgeless_actor(
        id = "game_instance_{}".format(instance["id"]),
        klass = GameOfLife,
        annotations = {
            "node_ids_allowed": id_str(instance["id"]),
            "init-payload": init_payload(instance)
        }
    ) for instance in instances
}

displays = {
    instance["id"]: edgeless_resource(
        id = "led-matrix-1-{}".format(instance["id"]),
        klass = LedMatrix,
        configurations = {},
        annotations = {
            "node_ids_allowed": id_str(instance["id"])
        }
    ) for instance in instances
}


# Display Connections
[ game_instances[instance["id"]].drawable >> displays[instance["id"]].update for instance in instances ]

# Trigger
game_instances[1].iteration_clock_o >> [game_instances[instance["id"]].iteration_clock_i for instance in instances ]
# [ game_instances[instance["id"]].iteration_clock_o >> game_instances[instance["id"]].iteration_clock_i for instance in instances ]

# Sides
[
    game_instances[instance["id"]].update_left_o >> game_instances[instance["left"]].update_right_i if instance["left"] else None for instance in instances
]
[
    game_instances[instance["id"]].update_right_o >> game_instances[instance["right"]].update_left_i if instance["right"] else None for instance in instances
]
[
    game_instances[instance["id"]].update_top_o >> game_instances[instance["top"]].update_bottom_i if instance["top"] else None for instance in instances
]
[
    game_instances[instance["id"]].update_bottom_o >> game_instances[instance["bottom"]].update_top_i if instance["bottom"] else None for instance in instances
]

# Corners
[
    game_instances[instance["id"]].update_top_left_o >> game_instances[instance["top_left"]].update_bottom_right_i if instance["top_left"] else None for instance in instances
]
[
    game_instances[instance["id"]].update_top_right_o >> game_instances[instance["top_right"]].update_bottom_left_i if instance["top_right"] else None for instance in instances
]
[
    game_instances[instance["id"]].update_bottom_left_o >> game_instances[instance["bottom_left"]].update_top_right_i if instance["bottom_left"] else None for instance in instances
]
[
    game_instances[instance["id"]].update_bottom_right_o >> game_instances[instance["bottom_right"]].update_top_left_i if instance["bottom_right"] else None for instance in instances
]

wf = edgeless_workflow(
    "game_of_life",
    game_instances.values() + displays.values(),
    annotations = {
        "feature_flags": "disable_ip_multicast_dialect",
    }
)

el_main = wf
