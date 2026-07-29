# Compactor Architecture

## Purpose

Compactor owns one request path: validate an HTTP request, resolve a canonical URL,
return the configured redirect, and describe the completed transaction. Source
configuration and event consumption are deliberately external concerns.

## Components

- `domain` owns validated identifiers, URLs, statuses, events, and the
  `RedirectSource` and `RedirectEventSink` contracts. It has no Axum or file-format
  knowledge.
- `source::json` validates one complete JSON document at startup and exposes an
  immutable in-memory lookup.
- `sink::jsonl` serializes append-only writes behind an asynchronous mutex and
  flushes each complete record.
- `http` reconstructs public request identity, captures bounded request metadata,
  orchestrates lookup, constructs events, and maps outcomes to HTTP.
- `config` turns `COMPACTOR_` environment variables into validated runtime state.
- `main` assembles adapters, binds the listener, and coordinates graceful shutdown.

## Request lifecycle

```text
socket peer + HTTP request
  → trusted-proxy and request validation
  → canonical scheme/authority/path
  → RedirectSource::resolve
  → response and sanitized RedirectEvent
  → RedirectEventSink::emit
  → HTTP response
```

The result is determined before event persistence. A sink failure is operationally
logged and cannot change a valid redirect, not-found, or client-error response.
Duration includes validation, lookup, and response construction but excludes sink
latency.

## Contracts and ownership

`RedirectSource` is synchronous because the reference lookup is immutable memory.
Its error-capable contract still admits fallible adapters. `RedirectEventSink` is
asynchronous because persistence is I/O. Both are object-safe so assembly and tests
can substitute adapters independently.

The HTTP layer owns `Location` and other protocol-sensitive headers. The source
owns redirect identity and permitted response metadata. The HTTP layer builds the
complete event; a sink never sees the request or invents timestamps.

## Security and privacy decisions

Forwarding headers are ignored unless the immediate socket peer is in an explicit
trusted CIDR. Trusted chains are walked from the nearest hop toward the client.
Malformed trusted metadata is rejected rather than producing a cross-tenant
lookup. Request-header capture uses a fixed allowlist and byte limits; credentials
and cookies never enter events.

## Extension points and tradeoffs

New sources implement `RedirectSource`; new event destinations implement
`RedirectEventSink`. Adapters should not depend on one another. A synchronous
source keeps v0.1 simple, while a future network source may justify evolving that
contract. Flushing each JSONL record favors observable durability over throughput;
rotation and batching belong in later adapters or external tooling.

The health path is reserved and bypasses redirect/event processing so an operator
can test service availability without exposing configuration.
