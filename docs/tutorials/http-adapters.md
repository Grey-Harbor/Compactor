# Run Compactor with HTTP adapters

Use this tutorial to evaluate remote redirect resolution and event delivery without
building another service first. The bundled mock implements both provider-neutral
HTTP contracts and keeps all state in the example process.

Start the mock in one terminal:

```sh
cargo run --example http_adapter_mock
```

Start Compactor in another terminal:

```sh
COMPACTOR_SOURCE_TYPE=http \
COMPACTOR_HTTP_SOURCE_URL=http://127.0.0.1:9090/resolve \
COMPACTOR_EVENT_SINK_TYPE=http \
COMPACTOR_HTTP_EVENT_SINK_URL=http://127.0.0.1:9090/events \
cargo run
```

Compactor warns because this local exercise uses plaintext HTTP. Request the one
redirect served by the mock:

```sh
curl -i -H 'Host: go.example' http://127.0.0.1:8080/docs
```

The response is a `308` to `https://docs.example.com/current/`, and the mock's
terminal prints the received event ID and outcome. A different path produces a
`404` and another event.

The first `/docs` request performs an authoritative HTTP lookup. Later requests
use `RedirectRuntime` according to the normal cache policy; the adapter does not
cache or retry. Stop both processes with Ctrl-C.

For credentials and production endpoint configuration, continue with
[Configure HTTP adapters](../how-to/configure-http-adapters.md). The exact wire
contract is in [HTTP adapter protocol](../reference/http-adapter-protocol.md).
