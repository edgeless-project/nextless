# Nextless Node

This crate provides the implementation of the Nextless worker node.

It is managed by the agent (`./src/agent`) that registers with a controller
and subsequently receives commands (e.g., to start/stop an actor) from this controller.

# Actor Runtimes

The worker node allows executing actors using one of three runtimes:

* Wasm (wasmtime): Our primary Wasm runtime based on Wasmtime.
  * This runtime provides support for GPU-acceleration using wgpu.
    * The actor-side implementation of our wgpu interface is provided by the crate `edgeless_function_gpu`.
* Wasm (wasmi): A secondary Wasm runtime based on the interpreter-based Wasmi.
  * This can be used on devices that do not allow for JIT-compilation.
* Native: An experimental runtime allowing for native execution using Rust/language-based isolation.
  * This relies on a custom ABI (`edgeless_function_abi`) and a userspace ELF loader.
  * The images are device-specific but OS-independent.
  * This requires additional work to be secure!
    * The compiler needs to enforce certain rules.
    * The node must ensure the binaries were built by a compiler enforcing those rules.
    * [https://www.usenix.org/conference/atc18/presentation/boucher](https://www.usenix.org/conference/atc18/presentation/boucher)
    * [https://dl.acm.org/doi/10.1145/3477113.3487272](https://dl.acm.org/doi/10.1145/3477113.3487272)

All of those runtimes rely on shared functionality found in `./src/base_runtime`.

# Resources

The node contains a set of resources found in `./src/resources`.

The current implementation does not contain a resource framework.
The resources are therefore implemented on a rather low level (directly interacting with the dataplane), and there is a lot of duplicate functionality.

# Dataplane

Actors and resources communicate using the shared dataplane.

`./src/dataplane` contains the implementation of the dataplane.  
This implementation handles the communication with components on local and remote nodes.  
The dataplane module also handles the port-name to recipient mapping.

Each node has a global `DataplaneProvider`.

Each actor/resource is represented by a `DataplaneHandle`.
This `DataplaneHandle` is created by the `DataplaneProvider`.
The handle is cloneable and internally runs a tokio task.

The dataplane in its current form is mostly designed around the overlay-based interactions.  
It also contains support for controller-defined dedicated links.
The remote overlay interactions are backed by the `InvocationAPI` of the `edgeless_api` crate.

# Telemetry / Metrics

Actors emit telemetry.  

The telemetry-related aspects can be found in `./src/telemetry`.  

Each actor is represented by a `TelemetryHandle`.  
The handles contain a set of tags and route the events towards an `EventProcessor`.

The implementation contains two main `EventProcessors`:

* `PrometheusEventTarget` exposed the events as Prometheus metrics served over HTTP.
* `EventLogger` outputs the events to stdout.
