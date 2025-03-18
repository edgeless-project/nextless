#load("../../functions/esp_test_fun/esp_test_fun.star", "EspTestFun")
load("../../functions/esp_test_fun_no_std/esp_test_fun_no_std.star", "EspTestFunNoStd")
load("../../resources/scd_30.star", "SCD30Sensor")
load("../../resources/epaper_display.star", "EPaperDisplay")

bridge = edgeless_actor(
    id = "bridge_i",
    klass = EspTestFunNoStd,
    annotations = {}
)

sensor = edgeless_resource(
    id = "scd30-sensor-1-1",
    klass = SCD30Sensor,
    configurations = {},
    # annotations = {}
)

display = edgeless_resource(
    id = "epaper-display-1-1",
    klass = EPaperDisplay,
    configurations = {},
    # annotations = {}
)

sensor.data_out >> bridge.measurement
bridge.message >> display.text

wf = edgeless_workflow(
    "esp_resource_test",
    [bridge, sensor, display],
    annotations = {}
)

el_main = wf
