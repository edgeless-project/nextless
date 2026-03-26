# Installation

## From Source

The component binaries can be built from source on Linux/MacOS.
A [Nix](https://nixos.org/) devshell containing the dependencies is provided (this works on Linux and MacOS).

```bash
# Optional, ensure all dependencies are met.
nix develop

# Build the node
cargo build --release

# Generate a node configuration
cargo run --release --bin=edgeless_node_d -- -t node.toml

#Run the node (alternatives)
cargo run --release --bin=edgeless_node_d
./target/release/edgeless_node_d

# Generate a controller configuration
cargo run --release --bin=edgeless_con_d -- -t controller.toml

#Run the controller (alternatives)
cargo run --release --bin=edgeless_con_d
./target/release/edgeless_con_d

# Generate a CLI configuration
cargo run --release --bin=edgeless_cli -- -t cli.toml

#Use the CLI (alternatives)
cargo run --release --bin=edgeless_cli
./target/release/edgeless_cli
```

## Playground Container

We provide a container that automatically starts a node and controller and provides an ephemeral development environment with `edgeless_cli`, the Rust toolchain, and other development tools.  
This container provides an easy way to quickly experiment with nextless.  
*The data in this container will be lost once the container is stopped!*

```bash
docker run -it --rm ghcr.io/edgeless-project/nextless_playground
```

You can use this container to follow the [quickstart guide](./quickstart.md).

You can attach to the container using VSCode.  
The development environment can be found at `/development`.
