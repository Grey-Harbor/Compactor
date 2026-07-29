# Configuration Reference

| Variable | Default | Meaning |
| --- | --- | --- |
| `COMPACTOR_BIND_ADDRESS` | `0.0.0.0:8080` | Listener socket address |
| `COMPACTOR_REDIRECTS_FILE` | `./redirects.json` | JSON redirect source |
| `COMPACTOR_EVENTS_FILE` | `./events.jsonl` | Append-only event output |
| `COMPACTOR_TRUSTED_PROXIES` | empty | Comma-separated IP/CIDR allowlist |
| `COMPACTOR_RECORD_CLIENT_ADDRESSES` | `true` | Include resolved client IP |
| `COMPACTOR_MAX_CAPTURED_HEADER_VALUE_BYTES` | `1024` | Per-value UTF-8 byte limit |
| `COMPACTOR_MAX_CAPTURED_HEADER_TOTAL_BYTES` | `4096` | Total captured-header value bytes |

`RUST_LOG` controls structured operational log filtering and defaults to `info`.
Malformed socket addresses, CIDRs, booleans, zero limits, an unreadable source, an
invalid source definition, or an unusable event file all prevent startup.
