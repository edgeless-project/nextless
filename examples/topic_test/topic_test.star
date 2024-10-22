load("../../functions/ping_async/pinger.star", "Pinger")
load("../../functions/pong_async/ponger.star", "Ponger")

pinger = edgeless_actor(
    id = "pinger_i",
    klass = Pinger,
    annotations = {}
)

ponger = edgeless_actor(
    id = "ponger_i",
    klass = Ponger,
    annotations = {}
)

ponger2 = edgeless_actor(
    id = "ponger_i_2",
    klass = Ponger,
    annotations = {}
)

pinger.ping >> topic("/foo/bar")
ponger.ping << topic("/foo/bar")
ponger2.ping << topic("/foo/bar")

ponger.pong >> pinger.pong
ponger2.pong >> pinger.pong

wf = edgeless_workflow(
    "ping_pong_async",
    [pinger, ponger, ponger2],
    annotations = {}
)

el_main = wf