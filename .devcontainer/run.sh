#!/bin/bash

docker run -td --name=edgeless_devcontainer \
--network=edgeless --network-alias=devcontainer \
--cap-add=SYS_PTRACE --cap-add=NET_ADMIN --security-opt seccomp=unconfined \
--device=/dev/net/tun -p 7021:7021 -p 7050:7050/udp -p 7002:7002 -p 7002:7002/udp -p 7011:7011 \
--mount type=bind,src=$PWD,dst=/app --mount type=volume,target=/app/target \
-u edgeless \
-w /app \
edgeless_dev