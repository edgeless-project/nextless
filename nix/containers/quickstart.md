# Nextless Playground
This is your development environment.
You can use `edgeless_cli` and `cargo generate` here.

# Quickstart: Simple HTTP Handler

Generate the Actor:
```bash
cargo generate --git https://github.com/edgeless-project/nextless templates/actor
```
Select `true` when prompted whether you want to genenerate an HTTP handler.

Build the Actor:
```bash
edgeless_cli function build example_actor_1/example_actor_1.star
```
`example_actor_1` corresponds to the project name selected in the previous step.

Generate the Application:
```bash
cargo generate --git https://github.com/edgeless-project/nextless templates/application
```
Select `true` when prompted whether you want to include a HTTP resource.

Start the Application:
```bash
edgeless_cli workflow start example_app_1/example_app_1.star
```
`example_app_1` corresponds to the project name selected in the previous step.
After this command has been executed, you should see activity in the controller and node panes.

Interact with the Application:
```bash
curl -H "Host: demo.localhost" http://127.0.0.1:7035/hello
```
