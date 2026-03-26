# System Components

There is three main components to each Nextless deployment.

1) The worker node (`edgeless_node`),
2) The controller (`edgeless_con`),
3) and the development environment containing the CLI tool (`edgeless_cli`).

A cluster contains one or more worker nodes and a controller.  
The controller has a runtime dependency on a working installation of Rust nightly to build Actor images for the worker nodes.

Each developer requires the CLI tool to interact with this controller.  
The CLI has a runtime dependency on a working installation of Rust nightly to build Actor images for the worker nodes.

The node registers with the controller (using the IP and port from the `controller_url` option in the node configuration).  
The controller interacts with the node using the Agent API (using the IP and port from the `agent_url_announced` option in the node configuration, which is passed to the controller during registration).  
It is important to ensure this bidirectional connectivity between the controller and nodes, e.g., when using containers.

Two nodes interact using the Interaction API (using the IPs and port from the `invocation_url_announced` and `invocation_url_announced_coap` options in the node configuration, which is passed to and distributed to the peers by the controller).

The CLI tool interacts with the controller (using the IP and port from the `controller_url` option in the CLI configuration).
