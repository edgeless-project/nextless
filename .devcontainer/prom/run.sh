#!/bin/bash

docker run -td --name=edgeless_prom \
    --network=edgeless --network-alias=prom \
    -p 9090:9090 \
    -v $PWD/prometheus.yml:/etc/prometheus/prometheus.yml \
    prom/prometheus \
    --config.file=/etc/prometheus/prometheus.yml \
    --web.enable-remote-write-receiver