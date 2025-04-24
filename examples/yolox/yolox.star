load("../../functions/yolox_wonnx/yolox.star", "YoloxNano")
load("../../functions/image_detection_tester/detection_tester.star", "DetectionTester")

yolox_example = edgeless_actor(
    id = "yolox_nano_i",
    klass = YoloxNano,
    annotations = {}
)

detection_tester = edgeless_actor(
    id = "tester_i",
    klass = DetectionTester,
    annotations = {}
)

detection_tester.test_image >> yolox_example.image
yolox_example.detection >> detection_tester.detection

wf = edgeless_workflow(
    "yolox_example",
    [detection_tester, yolox_example],
    annotations = {}
)

el_main = wf
