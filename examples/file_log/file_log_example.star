load("../../functions/message_generator/message_generator.star", "MessageGenerator")
load("../../resources/file_log.star", "FileLog")

logger = edgeless_resource(
    id = "my-log",
    klass = FileLog,
    # annotations = {},
    configurations = {
        "filename": "my-local-file.log",
        "add-timestamp": "true"
    }
)

generator = edgeless_actor(
    id = "my-message-generator",
    klass = MessageGenerator,
    annotations = {}
)

generator.message >> logger.line

wf = edgeless_workflow(
    "file_log_example",
    [logger, generator],
    annotations = {}
)

el_main = wf