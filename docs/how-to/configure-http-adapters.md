# Configure HTTP adapters

Use this guide when an authoritative service should resolve redirects, receive
events, or do both. Source and sink selection is independent; keep the JSON or
JSONL side when only one remote integration is needed.

Select the source and provide its complete endpoint URL:

```sh
export COMPACTOR_SOURCE_TYPE=http
export COMPACTOR_HTTP_SOURCE_URL=https://control.example/v1/resolve?tenant=public
```

Select the event sink independently:

```sh
export COMPACTOR_EVENT_SINK_TYPE=http
export COMPACTOR_HTTP_EVENT_SINK_URL=https://events.example/v1/redirect-events
```

Add a bearer token directly or through a file, never both. A file mounted by a
secret manager is preferable in production:

```sh
export COMPACTOR_HTTP_SOURCE_BEARER_TOKEN_FILE=/run/secrets/source-token
export COMPACTOR_HTTP_EVENT_SINK_BEARER_TOKEN_FILE=/run/secrets/sink-token
```

Compactor removes one final LF or CRLF from each token file. It rejects empty
tokens and never prints credential values. Rotate a token by replacing its file
and restarting Compactor; environment and token-file configuration is read only
at startup.

Static integration headers are JSON objects with string values:

```sh
export COMPACTOR_HTTP_SOURCE_STATIC_HEADERS_JSON='{"X-Tenant":"public"}'
export COMPACTOR_HTTP_EVENT_SINK_STATIC_HEADERS_JSON='{"X-Stream":"redirects"}'
```

Do not infer tenant names, routes, credentials, or authorization policy. Those
are deployment decisions. Compactor validates header syntax and rejects transport
and protocol headers it owns. A static `Authorization` value is allowed only when
bearer authentication is not configured.

Tune bounded calls only after measuring the dependency:

```sh
export COMPACTOR_HTTP_CONNECT_TIMEOUT_MS=500
export COMPACTOR_HTTP_SOURCE_REQUEST_TIMEOUT_MS=1500
export COMPACTOR_HTTP_SOURCE_MAX_RESPONSE_BYTES=65536
export COMPACTOR_HTTP_EVENT_SINK_REQUEST_TIMEOUT_MS=1500
```

Use HTTPS outside an isolated development network. HTTP is accepted for local or
private deployments but produces a startup warning because tokens and payloads
would travel without TLS protection. Custom CAs, mTLS, OAuth, and disabled TLS
verification are not supported.

Roll out by configuring one adapter at a time and observing source-error and sink-
error logs. Roll back by restoring its previous `*_TYPE` and file configuration,
then restart. `/healthz` remains local and does not test remote dependencies.
See [HTTP adapter protocol](../reference/http-adapter-protocol.md) for response
mapping and [Production readiness](prepare-for-production.md) for ownership.
