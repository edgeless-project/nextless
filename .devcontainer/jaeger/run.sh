docker run -td --name=edgeless_jaeger \
--network=edgeless --network-alias=jaeger \
-p 127.0.0.1:16686:16686 \
jaegertracing/jaeger:2.0.0 \
--set=receivers.otlp.protocols.grpc.endpoint="0.0.0.0:4317" --set=receivers.otlp.protocols.http.endpoint="0.0.0.0:4318"