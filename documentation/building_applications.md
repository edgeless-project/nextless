# Building Applications

An Application is defined using a Starlark-based Application description:

```star
# Load Actor / Resource Descriptions
load("../path_to_actor/demo_filter.star", "DemoFilter")
load("../path_to_resources/http_ingress.star", "HTTPIngress")

# Create a Logical Actor
filter = edgeless_actor(
    id = "filter",
    klass = DemoFilter,
    annotations = {
        "scaling_mode": "all_nodes",
        "node_ids_allowed": "00000000-0000-0000-0000-000000000001"
    },
)

# Create a Logical Resource
ingress = edgeless_resource(
    id = "demo_ingress",
    klass = HTTPIngress,
    configurations = {
        "host": "demo.localhost",
        "methods": "GET"
    },
    annotations = {
        "scaling_mode": "singleton"
    },
)

sensor = edgeless_actor(..)
dashboard = edgeless_actor(..)

# Link together input and output ports
sensor.output_name >> filter.input_name
ingress.new_request >> dashboard.http_in
filter.output_name >> topic("valid_measurements")
dashboard.data_in << topic("valid_measurements")

wf = edgeless_workflow(
    # Workflow Name
    "sensor_dashboard_demo",
    # Logical Services contained in this Application
    [sensor, filter, dashboard, ingress],
    
    annotations = {}
)

el_main = wf

```

One can bootstrap a new Application using [cargo-generate](https://github.com/cargo-generate/cargo-generate):
```
cargo generate --git https://github.com/edgeless-project/nextless templates/application
```

The developers need to include any Actor Class description they want to use in their application (cf. [building_actors.md](building_actors.md)).  
Furthermore, they need to include the Resource descriptions of Resources they want to include in their application.
Those can be found [here](https://github.com/edgeless-project/nextless/tree/development/resources).

For each logical Actor/Resource, there needs to be an instance created by a call to `edgeless_actor`/`edgeless_resource` that is also added to the list in `edgeless_workflow`.

## Actor Instances

In the call to `edgeless_actor`, the developers need to assign a logical id, link the actor to an Actor Class, and provide a set of `annotations`:
* `"init-payload": "String"` assigns the payload passed to the init call.
* `"scaling_mode": {}`: can be set to one of:
  * `singleton`: There will be only one physical instance of the actor.
  * `scalable`: Also requires `"min_instances": "10"` and `"max_instances": "25"` and allows the orchestrator to dynamically scale the number of instances based on the load.
  * `all_nodes`: Tries to place an instance of the actor on all nodes matching the constraints.

There are various annotations limiting the placement of the logical instances:
* `"node_ids_allowed": "uuid,uuid"`: Limits the instances to the listed node uuids.
* `"node_ids_denied": "uuid,uuid"`: Prevents instances from being spawned on the listed node uuids.
* `"runtime_dialects_allowed": "WASM,NATIVE_DYNAMIC"`: Limits the instance to use one of the listed runtimes.
* `"runtime_dialects_denied": "WASM,NATIVE_DYNAMIC"`: Prevents instances from using one of the listed runtimes.
* `"node_label_filter_allowed": "a&b&c|d&e|f"`: Boolean logic on the existence of node-labels. An instance can only be placed if the filter matches.
* `"node_label_filter_denied": "g&h&i|j&k|l"`: Boolean logic on the existence of node-labels. An instance won't be placed if the filter matches.

## Resource Instances

With the exception of the `init-payload`, those annotations also exist when creating a Resource instance with `edgeless_resource`.
For the Resources, the developers need to fill the parameter `configurations`. The configuration differs between the Resource Classes.

We ship multiple Resource Classes, including the following:

* `HTTPIngress`:
  * Ports:
    * `call_output("new_request", "eft.http.request", "eft.http.response")`
  * Configurations:
    * `"host": "demo.foo.bar"`: This is checked against the host-header.
    * `"methods": "GET,POST"`
* `FileLog`:
  * Ports:
    * `call_input("line", "String")`
  * Configurations:
    * `"filename": "my-local-file.log"`
    * `"add-timestamp": "true"`
* `LedMatrix`:
  * Ports:
    * `cast_input("update", "eft.led_matrix.matrix_frame")`
* `HTTPEgress`:
  * Ports:
    * `call_input("new_request", "eft.http.request", "eft.http.response")`

## Linking Ports

The Ports of Actors/Resources need to be linked together.
Unlinked ports may result in the removal of Actors/Resources.

There are multiple options to do this:

### Unicast

```star
source.port_o >> destination.port_i
```

Map the output to a *single* instance of `destination`.
The instance is picked at orchestration time.

### Anycast

```star
source.port_o >> any([destination.port_i, destination_2.port_i])
```

Map the output to all of the instances of the destinations.
Each message will only be sent to a single instance of the destinations.

### Multicast

```star
source.port_o >> [destination.port_i, destination_2.port_i]
```

Map the output to all of the instances of the destinations.
Each message will be sent to all instances of the destinations.

### Topic

```star
source.port_o >> topic("topic_name")
destination.port_id << topic("topic_name")
```

Orchestration-time abstraction around Multicast.
Each message will be sent to each instance of an Actor/Resource whose input is mapped to the same topic.
