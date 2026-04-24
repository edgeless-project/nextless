# Quickstart: Simple HTTP Handler

In this quickstart guide, we go through the process of defining an Actor and Application and starting it on a Nextless cluster.

This quickstart guide assumes a running installation of the controller and a node with an HTTP Ingress Resource.  
It also assumes you are in a directory next to a directory `nextless` containing the project's source code---this is used for the Resource definitions.  
Alternatively, you can change the paths to the Resource definitions in the Application description.

You can generate an ephemeral environment fulfilling these requirements using our playground container:
```bash
docker run -it --rm ghcr.io/edgeless-project/nextless_playground
```
Please refer to the [installation guide](./installation.md) for alternative options and additional details. 

# Create and Build the Actor:

In the first step, we will use a template to generate an Actor.  
The guide assumes you selected `example_actor_1` as the project name.  
Leave the input and output ports empty.
Select `true` when prompted whether you want to generate an HTTP handler.  
Select `false` when prompted whether you want to generate an output for the LED Matrix.

```bash
cargo generate --git https://github.com/edgeless-project/nextless templates/actor
```

The above command generates a directory with the Actor's definition and Rust-based implementation.  
You may explore this directory (please refer to [building_actors.md](./building_actors.md) for more information) or proceed to the next step.

You can build the Actor using the following command (`example_actor_1` corresponds to the project name selected in the previous step):
```bash
edgeless_cli function build example_actor_1/example_actor_1.star
```

This will compile the Actor to Wasm.

# Create and Build the Application: 

Similar to the Actor, you can generate the Application Description using a template.  
The project name must be different from the name chosen for the Actor.  
Set the Actor name to the project name from the last step.  
Select `true` when prompted whether you want to include an HTTP Resource.  
Select `false` when asked whether you want to include an LED Matrix Resource.

```bash
cargo generate --git https://github.com/edgeless-project/nextless templates/application
```

This command generates a directory containing the Application description.  
You may explore this directory (please refer to [building_applications.md](./building_applications.md) for more information) or continue to the next step.

You can start the Application using the following command (`example_app_1` corresponds to the project name selected in the previous step):
```bash
edgeless_cli workflow start example_app_1/example_app_1.star
```

After this command has been executed, you should see activity in the logs of both the controller and node.

# Interact with the Application:

You can send an HTTP request to the Application using the following command:

```bash
curl -w "\n" -H "Host: demo.localhost" http://127.0.0.1:7035/hello
```

You should receive the response `Handler not configured`.  
Additionally, you should see log output from the node.
