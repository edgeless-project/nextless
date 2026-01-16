load("../../functions/yolox_wonnx/yolox.star", "YoloxNano")
load("../../functions/image_detection_tester/detection_tester.star", "DetectionTester")
load("../../functions/image_scaler/scaler.star", "ImageScaler")

yolox_example = edgeless_actor(
    id = "yolox_nano_i",
    klass = YoloxNano,
    annotations = {
        "scaling_mode": "all_nodes"
    }
)

detection_tester = edgeless_actor(
    id = "tester_i",
    klass = DetectionTester,
    annotations = {
        "scaling_mode": "singleton"
    }
)

scaler = edgeless_actor(
    id = "scaler_i",
    klass = ImageScaler,
    annotations = {
        "scaling_mode": "all_nodes",
    }
)

detection_tester.test_image >> scaler.unscaled
scaler.scaled >> yolox_example.image
yolox_example.detection >> detection_tester.detection

wf = edgeless_workflow(
    "yolox_example",
    [detection_tester, yolox_example, scaler],
    annotations = {}
)

el_main = wf
