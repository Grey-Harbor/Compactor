# Configuration Reference

Compactor reads configuration from the environment once during startup. Unset
variables use their documented defaults. An explicitly empty value is parsed as
configuration, so it may fail validation; `COMPACTOR_TRUSTED_PROXIES` is the
exception, where an empty value means no trusted proxies.

| Variable | Default | Accepted value and effect |
| --- | --- | --- |
| `COMPACTOR_BIND_ADDRESS` | `0.0.0.0:8080` | A valid IP socket address. Compactor binds only after other startup validation succeeds. |
| `COMPACTOR_REDIRECTS_FILE` | `./redirects.json` | Path to the complete JSON redirect source. The file must exist and validate. |
| `COMPACTOR_EVENTS_FILE` | `./events.jsonl` | Path to the append-only event file. Its parent directory must already exist and be writable. |
| `COMPACTOR_TRUSTED_PROXIES` | empty | Comma-separated IP addresses or CIDRs. Whitespace and empty entries are ignored. |
| `COMPACTOR_RECORD_CLIENT_ADDRESSES` | `true` | Exactly `true` or `false`. When false, `client.address` is always null. |
| `COMPACTOR_MAX_CAPTURED_HEADER_VALUE_BYTES` | `1024` | Positive integer UTF-8 byte limit applied to each captured value. |
| `COMPACTOR_MAX_CAPTURED_HEADER_TOTAL_BYTES` | `4096` | Positive integer budget across captured request-header values. |
| `COMPACTOR_REDIRECT_CACHE_TTL_SECONDS` | `300` | Positive integer freshness lifetime for a successfully resolved redirect. |
| `COMPACTOR_REDIRECT_CACHE_MAX_ENTRIES` | `10000` | Positive integer maximum number of resident redirect definitions. |

`RUST_LOG` controls structured operational log filtering through
`tracing-subscriber` and defaults to `info`. For example:

```sh
RUST_LOG=compactor=debug cargo run
```

Startup fails before traffic is accepted when a socket address, trusted proxy,
boolean, TTL, or limit is malformed; a TTL or limit is zero; the source cannot be
read or validated; the event file cannot be opened; or the listener cannot bind.

Environment configuration is not reloaded. Restart after changing an environment
value. The source file is different: it is reread on a cold lookup or stale
refresh, so an atomically installed valid replacement takes effect according to
the configured cache TTL without a restart. See
[Configure the JSON redirect source](../how-to/configure-json-source.md) for
rollout and rollback behavior.
