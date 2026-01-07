# Nextless: Compiler-Inspired Serverless

This repository contains a research prototype of Nextless, a "next-generation" serverless platform for edge computing scenarios.

Nextless relies on
* *compiler-inspired transformations and optimizations* in the orchestration system (orchestration-time abstractions),
* the integration of *resources such as sensors and actuators*,
* and the use of *lightweight runtimes*,

to enable the efficient execution of complex serverless applications on sets of heterogeneous and potentially resource-constrained edge devices.

Nextless is a research fork of the [EDGELESS reference implementation](https://github.com/edgeless-project/edgeless).

## Overview

Nextless executes *Applications* comprising *Actors* and *Resources*.

Actors represent stateful but side-effect-free services defined by untrusted developers.
Due to their untrusted nature, they are executed in an isolated way using suitable isolation techniques, e.g., process VMs.

Resources are included in the implementation of the worker nodes and can only be configured and instantiated.
They are executed without any sandbox and are thereby able to cause external effects and may be triggered by external effects.

Both Actors and Resources define a set of input and output *Ports*.
Those are mapped together in the *Application Description*, enabling the reuse of Actors across applications.

Both the Actor's logic as well as the mappings between Ports can be defined using a set of formats called *Dialects* (a term inspired by MLIR).
The platform's orchestration system can convert between these dialects.

The conversion of the Actor's logic allows the system to efficiently target heterogeneous worker nodes, e.g., using platform-specific native images.

The conversion of Port mappings enabled the developers to use abstractions that are then efficiently mapped to the available links.
As an example, the developers may use pub/sub, which is then automatically converted to IP Multicast.

In addition to the conversions, the system can use the holistic view provided by the application description for optimizations.
As an example, it can use static analysis to detect and remove unused links, actors, and parts of the actors before they are deployed onto the worker nodes.

In addition to the static optimizations, the system also tracks the runtime performance of the actor instances and uses this knowledge for adaptive optimizations.

## License

The Repository is licensed under the MIT License. Please refer to
[LICENSE](LICENSE) and the copyright headers in each file. 

## Funding & Heritage
This project started as a fork of the [EDGELESS Reference Implementation](https://github.com/edgeless-project/edgeless)
and is developed as part of the EDGELESS Project.

The fork's main focus and differentiating factor is the integration of compiler-inspired transformations, which also require a more well-defined application model.

The fork's initial focus lies on independent and locally-aggregated clusters of devices, with support for multi-cluster deployments and cloud backends deferred for later.
Compared to the EDGELESS reference implementation, Nextless simplifies the control plane by merging the controller and orchestrator and
will rely on the federation of controllers and the deployment of sub-applications to span multiple clusters.


EDGELESS received funding from the [European Health and Digital Executive Agency
 (HADEA)](https://hadea.ec.europa.eu/) program under Grant Agreement No 101092950.



![](documentation/edgeless-logo-alpha-200.png)
