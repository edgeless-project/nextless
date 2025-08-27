### FileLog example

The example creates a function that periodically send messages to be saved to a file.

First, package the `message_generator` function.

Then you can start and stop the workflow with:

```
ID=$(target/debug/edgeless_cli workflow start examples/file_log/file_log_example.star)
target/debug/edgeless_cli workflow stop $ID
```
