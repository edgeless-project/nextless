# Nextless Controller

This crate contains the Nextless Controller.

It can be grouped into the service-related parts (comprised of `crate::controller::*`, the library root, and the `edgeless_con_d` binary),
and the model and transformation engine (`crate::ir::*`).  

The engine represents the core of this crate, but is intentionally kept passive:
It is only active when triggered by the service and does not directly interact with the worker nodes to materialize changes.  
This decoupling is intentional and may allow for building a simulator around the engine component.

This README only provides a basic technical overview of this crate.  
There exists a work-in-progress paper draft further discussing the design of the compiler-inspired orchestration system.  
This draft is not in a publishable state and more information is available upon request.

## IR and Transformation Engine

The compiler-inspired aspects of this project are implemented by the engine component.

### Applications (Workflows)

An Application is represented by an instance of `crate::ir::workflow::ActiveWorkflow`, which contains both logical and physical component instances.  
This `ActiveWorkflow` represents the state of this application across the logical and physical model.

It is wrapped by an instance of `crate::ir::managed_workflow::ManagedWorkflow` that combines it with a pipeline of transformations.  
This `ManagedWorkflow` acts as the entry point into the engine and provides methods that are called when the application is initially spawned,
stopped, or when the transformations need to be reapplied to perform optimizations or adapt to changes in the system model.  
Those methods are called by the service and only return the changes to be materialized, and do not directly apply these changes to the worker nodes.

### Logical and Physical Instances

There are two primary types of components: `Actors` and `Resources`.  
The repository contains initial work related to the two component types `Proxy` and `SubFlow` that are not in an useable state.  
There are two main representations of each component: The logical and the physical instances.  
Logical instances represent the intended state of the components in an abstract way,
while the physical instances represent the instances of the components.

Physical instances exist in different lifecycle states:  
* There might be the request to spawn a new instance,
* the placement logic might have decided on where the instance should be placed,  
* the instance is already materialized and enriched with runtime telemetry (materialized instances),
* or the instance was materialized at some point but is not active anymore.

The engine wraps the physical instances in a state machine (`crate::ir::physical_model::PhysicalComponentState`) that tracks their lifecycle.

### Transformations

The set of both logical and physical instances is modified by the transformations (`crate::ir::transformations::*`).  
They are grouped into  
* the Logical Phase logically optimizing the requested application input (represented by `crate::ir::pipeline::default_logical::DefaultLogicalPipeline`) and   
* the Physical Phase handling typical orchestration tasks such as scaling and placement and the optimization of the application instances.

The Physical Phase is represented by `crate::ir::pipeline::default_orchestration::DefaultOrchestrationPipeline` and `crate::ir::pipeline::default_physical::DefaultPhysicalPipeline`.

A new transformation needs to implement one of the transformation traits (e.g. `crate::ir::transformations::StatelessPhysicalTransformation`) and needs to be added to the transformation pipeline.

The translation of interaction and behavior dialects represent important sets of transformations.  
Inspired by MLIR, instead of writing top-level transformations for each dialect,  
the engine contains the dialect conversion frameworks `crate::ir::behavior::*` and `crate::ir::interaction::*`.  
Dialects are added to the dialect registries and used in transformations such as the `LogicalInteractionNormalizer`.
