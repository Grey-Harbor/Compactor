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

`RUST_LOG` controls structured operational log filtering through
`tracing-subscriber` and defaults to `info`. For example:

```sh
RUST_LOG=compactor=debug cargo run
```

Startup fails before traffic is accepted when a socket address, trusted proxy,
boolean, or limit is malformed; a limit is zero; the source cannot be read or
validated; the event file cannot be opened; or the listener cannot bind.

Configuration is not reloaded. Restart the process after changing any value or the
source file.
