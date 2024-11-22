#!/bin/bash

docker run -td --name=edgeless_otelco \
--network=edgeless --network-alias=otelco \
-p 127.0.0.1:4317:4317 \
-p 127.0.0.1:4318:4318 \
--mount type=bind,src=$PWD/config.yml,dst=/etc/otelcol-contrib/config.yaml \
otel/opentelemetry-collector-contrib:0.113.0

