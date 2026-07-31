# Configuration Reference

Use this reference to check exact environment names, defaults, and startup
validation. Compactor reads configuration once at startup. Adapter-specific
settings are validated only when that adapter is selected.

## Core settings

| Variable | Default | Accepted value and effect |
| --- | --- | --- |
| `COMPACTOR_BIND_ADDRESS` | `0.0.0.0:8080` | Valid IP socket address; binding occurs after adapter startup validation. |
| `COMPACTOR_SOURCE_TYPE` | `json` | Exactly `json` or `http`. |
| `COMPACTOR_EVENT_SINK_TYPE` | `jsonl` | Exactly `jsonl` or `http`; selection is independent of the source. |
| `COMPACTOR_TRUSTED_PROXIES` | empty | Comma-separated IP addresses or CIDRs. |
| `COMPACTOR_RECORD_CLIENT_ADDRESSES` | `true` | Exactly `true` or `false`. |
| `COMPACTOR_MAX_CAPTURED_HEADER_VALUE_BYTES` | `1024` | Positive integer per captured value. |
| `COMPACTOR_MAX_CAPTURED_HEADER_TOTAL_BYTES` | `4096` | Positive integer total captured-header budget. |
| `COMPACTOR_REDIRECT_CACHE_TTL_SECONDS` | `300` | Positive integer freshness lifetime for found redirects. |
| `COMPACTOR_REDIRECT_CACHE_MAX_ENTRIES` | `10000` | Positive integer resident-definition limit. |

## File adapters

| Variable | Default | Used when |
| --- | --- | --- |
| `COMPACTOR_REDIRECTS_FILE` | `./redirects.json` | Source type is `json`; the entire file must validate before listening and on every authoritative resolution. |
| `COMPACTOR_EVENTS_FILE` | `./events.jsonl` | Sink type is `jsonl`; its parent must exist and be writable. |

## Shared HTTP transport

| Variable | Default | Accepted value and effect |
| --- | --- | --- |
| `COMPACTOR_HTTP_CONNECT_TIMEOUT_MS` | `500` | Positive integer connection timeout, used when either HTTP adapter is selected. |

One reusable client provides connection pooling, rustls certificate verification,
and disabled redirect following. HTTP and HTTPS endpoint schemes are accepted;
selected plaintext endpoints produce a startup warning.

## HTTP source

These values apply only when `COMPACTOR_SOURCE_TYPE=http`.

| Variable | Default | Accepted value and effect |
| --- | --- | --- |
| `COMPACTOR_HTTP_SOURCE_URL` | none | Required absolute HTTP(S) URL without user information, fragment, or an existing `url` query parameter. |
| `COMPACTOR_HTTP_SOURCE_REQUEST_TIMEOUT_MS` | `1500` | Positive integer total lookup timeout. |
| `COMPACTOR_HTTP_SOURCE_MAX_RESPONSE_BYTES` | `65536` | Positive integer response limit, enforced incrementally. Encoded responses are rejected. |
| `COMPACTOR_HTTP_SOURCE_BEARER_TOKEN` | none | Non-empty bearer credential. Mutually exclusive with `_FILE`. |
| `COMPACTOR_HTTP_SOURCE_BEARER_TOKEN_FILE` | none | Token file path. One trailing LF or CRLF is removed. |
| `COMPACTOR_HTTP_SOURCE_STATIC_HEADERS_JSON` | `{}` | JSON object whose values are strings. |

## HTTP event sink

These values apply only when `COMPACTOR_EVENT_SINK_TYPE=http`.

| Variable | Default | Accepted value and effect |
| --- | --- | --- |
| `COMPACTOR_HTTP_EVENT_SINK_URL` | none | Required absolute HTTP(S) URL without user information or fragment. |
| `COMPACTOR_HTTP_EVENT_SINK_REQUEST_TIMEOUT_MS` | `1500` | Positive integer total POST timeout. |
| `COMPACTOR_HTTP_EVENT_SINK_BEARER_TOKEN` | none | Non-empty bearer credential. Mutually exclusive with `_FILE`. |
| `COMPACTOR_HTTP_EVENT_SINK_BEARER_TOKEN_FILE` | none | Token file path. One trailing LF or CRLF is removed. |
| `COMPACTOR_HTTP_EVENT_SINK_STATIC_HEADERS_JSON` | `{}` | JSON object whose values are strings. |

Static headers cannot override `Host`, `Content-Length`, `Connection`,
`Transfer-Encoding`, `TE`, `Trailer`, `Upgrade`, `Proxy-Authorization`, or each
adapter's protocol headers. A configured bearer token also reserves
`Authorization`; otherwise a static authorization header may implement another
fixed scheme. Credential values never appear in formatted configuration or
adapter errors.

`RUST_LOG` controls structured log filtering and defaults to `info`:

```sh
RUST_LOG=compactor=debug cargo run
```

Startup does not contact HTTP endpoints. It validates selected configuration,
builds one shared client when needed, opens selected files, then binds. `/healthz`
is local and dependency-free. Environment values and token files require restart
to reload; JSON redirect content is reread according to runtime cache behavior.

See [Configure HTTP adapters](../how-to/configure-http-adapters.md),
[HTTP adapter protocol](http-adapter-protocol.md), and
[Configure the JSON redirect source](../how-to/configure-json-source.md).
