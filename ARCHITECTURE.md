# Compactor Architecture

## Purpose and system boundary

Compactor is a small HTTP service with one responsibility: resolve an incoming
public URL to a configured destination, return the redirect, and describe the
completed transaction as a sanitized event. It is deliberately a request-time
component, not a URL-management product.

Configuration management owns creation and distribution of redirect definitions.
The reverse proxy owns TLS termination and replacement of client-supplied
forwarding metadata. Event collectors own rotation, retention, shipping, and
analysis. Compactor owns none of those concerns.

The project website is a separate static presentation surface. It reads the
repository's Markdown documentation at build time and publishes generated HTML;
it does not communicate with a running Compactor service or change the runtime
boundary.

The v0.2 goals are deterministic multi-host lookup, source-independent redirect
caching, correct redirect responses, bounded privacy-aware events, independently
replaceable source and sink adapters, and predictable startup and shutdown.
Management APIs, source mutation APIs, authentication, dashboards, analytics,
databases, metrics, event retry queues, batching, event rotation, and service
deployment are explicit non-goals.

## Context and data flow

```text
configuration system ── JSON file or remote API ──► source adapter
                                                        │ authoritative resolution
                                                        ▼
client ── proxy ── HTTP ──► request pipeline ──► redirect runtime/cache ──► HTTP response
                              │
                              └── event ──► JSONL file or HTTP sink ──► external collector
```

JSON and JSONL are reference adapters, not architectural requirements. The
application depends on domain contracts, so another source or event destination
can be assembled without coupling the adapters to each other.

## Website and documentation publishing

The `site/` project is a statically exported Next.js application. Its landing page
explains the product boundary, while its Fumadocs-based documentation routes render
the Markdown files under `docs/`. The Markdown remains canonical: the website
build discovers it, derives routes from its paths, rewrites internal Markdown links
to generated routes, and links files outside `docs/` back to GitHub.

The site has no runtime API, database, or state. `next build` produces the complete
`site/out` artifact, including canonical metadata, sitemap, robots policy, social
preview, and custom-domain file. CI type-checks and builds this project without
publishing it. A separate GitHub Pages workflow publishes the same static artifact
only after changes reach `main`, keeping deployment out of the service and
container release workflows.

## Components and dependency direction

- `domain` owns validated redirect IDs, event IDs, canonical URLs, constrained
  redirect statuses, permitted response headers, event values, explicit adapter
  errors, `RedirectSource`, and `RedirectEventSink`.
- `runtime` owns positive redirect residency, freshness, LRU eviction, cold-load
  single-flight, stale refresh coordination, and background-task shutdown.
- `source::json` stores a file path and resolves authoritative state by reopening,
  parsing, and validating the complete document.
- `source::http` performs one bounded authoritative GET and converts the returned
  record through the same private validation path as the JSON source.
- `sink::jsonl` serializes append-only asynchronous writes to one file.
- `sink::http` performs one bounded POST of the exact immutable event.
- `adapter_http` owns validated endpoints and credentials plus one pooled,
  redirect-disabled rustls client shared by selected HTTP adapters.
- `http` reconstructs public request identity, sanitizes metadata, asks the runtime
  to resolve a redirect, determines the response, and constructs the event.
- `config` converts `COMPACTOR_` environment variables into validated runtime
  values.
- `main` initializes logging and adapters, binds the listener only after startup
  validation, and coordinates signal-driven graceful shutdown.

The domain has no dependency on Axum or filesystem handling. It uses `http`
header primitives to validate protocol metadata, `url::Url` for destination
values, and Serde only for the public event and validated-value wire contracts.
Adapters depend inward on the domain; the domain never imports an adapter.

## Domain contracts and ownership

`RedirectSource` is object-safe and asynchronous:

```rust
#[async_trait]
pub trait RedirectSource: Send + Sync {
    async fn resolve(
        &self,
        canonical_url: &CanonicalUrl,
    ) -> Result<Option<RedirectDefinition>, RedirectSourceError>;
}
```

The explicit error distinguishes a source failure from a valid miss. Sources own
only authoritative resolution and external-record validation. They do not own
TTL, cache residency, refresh, eviction, request outcomes, or cross-request
coordination. The asynchronous contract permits file and future network-backed
sources without exposing backend-specific behavior to the runtime.

`RedirectRuntime` wraps one source and applies `RedirectCachePolicy`, whose public
inputs are a positive freshness TTL and a positive maximum resident-entry count.
HTTP depends on the runtime rather than the source. This keeps every file, HTTP,
database, or future custom source behind the same lifecycle.

`RedirectEventSink` is object-safe and asynchronous because emission may perform
I/O. The HTTP layer passes it a complete event. A sink does not inspect requests,
resolve clients, choose outcomes, or invent timestamps.

Redirect IDs are stable opaque source identities. Event IDs are independently
generated ULIDs and identify individual transactions. The source owns destination,
status, redirect ID, and permitted response metadata. The HTTP layer always owns
`Location`, response selection, request sanitization, and event construction.

## Canonical request identity

A canonical lookup key contains public scheme, host, non-default port, and path.
It intentionally excludes query and fragment. Its invariants are:

- scheme and host compare case-insensitively and serialize in lowercase;
- port `80` for HTTP and `443` for HTTPS is omitted;
- an empty path becomes `/`;
- user information, relative URLs, unsupported schemes, malformed hosts, and
  ambiguous backslash forms are rejected;
- path case, trailing slash, repeated separators, literal dot segments,
  percent-encoded dot segments, and percent-escape spelling are preserved.

Consequently `/a/../b`, `/b`, `/%2e%2e/b`, and `/~user` are distinct lookup
identities. This is intentional: Compactor must not silently reinterpret an
operator's tenant path. The same constructor is used for source configuration and
incoming requests so both sides share one normalization rule.

Three URL-shaped values remain distinct:

1. The incoming request event records reconstructed scheme and host plus the raw
   request path and query.
2. The canonical URL is the query-free normalized lookup identity.
3. The response event records the final destination after query forwarding.

This separation prevents canonical normalization from rewriting the observed
request.

## Startup and adapter lifecycle

Startup proceeds in dependency order:

1. Parse common settings and the independent source/sink selectors. Validate only
   settings belonging to selected adapters.
2. When either HTTP adapter is selected, validate its URL, credentials, headers,
   positive timeouts and limits, then construct one shared client. No endpoint is
   contacted during startup; plaintext endpoints produce a warning.
3. When JSON is selected, read and deserialize the entire source once for
   fail-fast validation, then discard it. Validate every record and global
   uniqueness invariant.
4. When JSONL is selected, open or create the event file and verify its parent is
   usable.
5. Construct an initially empty redirect runtime and bind the TCP listener.
6. Announce readiness with adapter descriptions, freshness TTL, and entry limit.

Any failure exits before traffic is accepted. Each authoritative file resolution
reopens and validates the complete document before returning one record. No valid
subset of a malformed document becomes visible. File replacements therefore take
effect without restarting, subject to cache freshness. Operators must install a
complete file atomically; an in-place partial write can cause cold lookups to fail
while resident redirects continue through stale serving.

Configured status is limited to `301`, `302`, `303`, `307`, and `308`.
`Location`, `Content-Length`, `Connection`, `Transfer-Encoding`, `Date`, and
`Server` are protocol-owned and rejected in source configuration. Other configured
headers must be valid HTTP names and values.

The HTTP source receives a canonical key and appends it as an encoded `url` query
parameter to its configured endpoint, preserving fixed query parameters. `200`
must provide an unencoded JSON media type and one valid record within the response
limit; `404` is authoritative absence; every other response or transport failure
is a source error. Redirects are not followed and incoming request headers are not
forwarded. The adapter has no retry or cache behavior, so runtime handling is
identical to file-source results.

## Redirect cache lifecycle

Only successful definitions become resident. `NotFound` and source errors are not
cached. Concurrent cold requests for one canonical URL share one source call and
receive the same result; unrelated keys resolve independently.

A resident entry is `Fresh`, `Refreshing`, or stale with a retry time. Freshness
uses Tokio's monotonic clock and begins when source resolution completes. The
first request at or after expiry atomically marks the entry `Refreshing`, starts
one background source call, and immediately returns the stale definition. Other
requests continue returning that definition and do not start more refreshes.

Refresh completion has three outcomes:

| Source result | Resident transition | Triggering HTTP response |
| --- | --- | --- |
| Found | Replace definition and become fresh for one TTL | Previously cached redirect |
| Not found | Remove the entry | Previously cached redirect, served one last time |
| Error | Keep stale data and defer retry for 30 seconds | Previously cached redirect |

There is no hard stale-age limit. After a refresh failure, one request at or after
the retry time may start another refresh. The failure is logged, but the request
event remains `redirected` because that is the response actually selected.

The cache holds at most the configured number of resident definitions and updates
recency on every hit. Insertion evicts the least-recently-used entry that is not
refreshing. A refresh in flight is never cancelled for eviction; if every resident
entry is refreshing, a new source result is returned to its callers without being
cached. In-flight cold-load coordination is transient and does not count as
resident redirect data.

## Request lifecycle

For every request except event-free health probes:

1. Start a monotonic timer and retain the socket peer, method, URI, protocol, and
   allowlisted metadata needed for an event.
2. If the immediate peer is trusted, parse forwarding metadata; otherwise ignore
   all forwarding headers.
3. Validate the reconstructed scheme and authority. A failure produces `400`
   without calling the source.
4. Reject methods other than `GET` and `HEAD` with `405` and
   `Allow: GET, HEAD`, also without calling the source.
5. Construct the query-free canonical URL and ask the redirect runtime to resolve it.
6. Determine the complete status, `Location`, configured headers, redirect ID,
   and outcome.
7. Stop the processing timer and build the event with a UTC timestamp.
8. Await event emission. The HTTP sink makes one bounded POST; log a sink error
   but do not change the response.
9. Return the previously determined response.

The event duration includes validation, lookup, and response construction. It
excludes sink latency and is serialized as finite, non-negative milliseconds.
Events are constructed after response determination, so their response fields
describe what the pipeline intends to return. `HEAD` mirrors `GET` status and
headers with an empty body.

| Condition | Status | Outcome | Runtime behavior | Redirect ID / location |
| --- | ---: | --- | --- | --- |
| Fresh, stale, or newly resolved definition | configured 3xx | `redirected` | return definition | both present |
| Authoritative miss | 404 | `not_found` | do not cache | both null |
| Invalid metadata or URL | 400 | `invalid_request` | not called | both null |
| Unsupported method | 405 | `invalid_request` | not called | both null |
| Cold source adapter error | 500 | `source_error` | do not cache | both null |

Sink failure is deliberately absent from the outcome table. Event persistence is
secondary to the correct HTTP result and is reported only through operational
logging. Sink latency is excluded from `duration_ms`, though awaiting a selected
HTTP sink can delay returning that response up to its configured total timeout.

## Query forwarding and response construction

Queries do not affect source lookup. When a definition matches, the configured
destination query remains first and the incoming raw query is appended after it.
Ordering, blank values, and duplicate names are preserved. If only one side has a
query, it is retained unchanged. Fragments are never forwarded because clients do
not send them to the server. URL-aware mutation builds the final destination;
Compactor never accepts a configured `Location` override.

## Proxy trust and client identity

Compactor serves plain HTTP. A trusted reverse proxy can reconstruct the public
HTTPS identity only when its immediate socket address belongs to
`COMPACTOR_TRUSTED_PROXIES`.

For trusted peers, RFC 7239 `Forwarded` has precedence. Missing fields fall back to
`X-Forwarded-Proto`, `X-Forwarded-Host`, and `X-Forwarded-For`; direct request
values are used only when the corresponding forwarded value is absent. Malformed
trusted metadata produces an invalid request rather than a possibly cross-tenant
lookup. Untrusted peers cannot influence public identity or client address through
forwarding headers.

The address chain is interpreted from client to nearest proxy. Compactor appends
the socket peer, walks from right to left past configured trusted proxies, and
records the nearest remaining address. The deployment proxy must replace
client-supplied forwarding headers rather than preserve an attacker-controlled
suffix. Client-address recording can be disabled independently of lookup.

## Privacy and event schema

Every non-health transaction gets one UTC RFC 3339 timestamp and one ULID event
ID. Request and response status information is recorded for all four outcomes.
The generic request-header map has a fixed normalized allowlist:
`referer`, `accept`, `accept-language`, and `x-request-id`. `User-Agent` has a
dedicated client field.

Each captured value is deterministically truncated at a valid UTF-8 byte boundary.
Fields are considered in the documented allowlist order and omitted when adding
the truncated value would exceed the total byte limit. Non-text header values are
omitted. Cookies, credentials, authorization values, and arbitrary headers are
never captured.

## JSONL concurrency and durability

The reference sink opens one file with create and append behavior. An asynchronous
mutex covers serialization of a complete JSON object, its newline, the write, and
the flush, so concurrent requests cannot interleave records. The lock also creates
backpressure: requests can complete processing concurrently but event writes queue
behind the file.

Every successful `emit` has flushed one complete JSONL record through Tokio's
buffer to the operating system. Compactor does not call `fsync`, promise survival
across power loss, repair a partial record after process or filesystem failure,
retry, rotate, batch, or ship data. Those responsibilities belong to a future sink
or external tooling.

## Health, observability, and shutdown

`GET /healthz` returns `200` with `ok\n`; `HEAD /healthz` returns the same status
with an empty body. Both bypass source lookup and event emission and disclose no
configuration, source data, events, metrics, or dependency details. Other methods
on `/healthz` use the normal `405` invalid-request path and emit an event.

Structured operational logs go to stderr and are filtered by `RUST_LOG`. Startup,
shutdown, source failures, malformed trusted metadata, and sink failures are
operational signals; request event data remains in the event adapter.

SIGINT and SIGTERM trigger Axum graceful shutdown: the listener stops accepting
new connections and in-flight requests may finish. The runtime then rejects new
lookups, drains source tasks already in flight, and only then logs process
completion. Source adapters must apply finite backend timeouts; Compactor does not
impose a separate refresh-drain timeout. Request-owned sink calls finish as part
of Axum's drain, so no separate HTTP sink queue or shutdown worker exists.

## Deployment topology

The production image builds with Rust 1.85 and runs as UID/GID `10001:10001`.
Redirect configuration is mounted read-only at `/etc/compactor/redirects.json`;
event storage is mounted writable at `/var/lib/compactor`. The container health
check calls the loopback health endpoint. TLS, public routing, forwarding-header
sanitization, persistent-volume ownership, event collection, and restart policy
remain deployment responsibilities.

## Verification strategy

Unit tests enforce validated types, raw-path canonical invariants, query merging,
header limits, proxy precedence, cold single-flight, state transitions, refresh
cooldown, bounded LRU residency, and refresh draining. Adapter tests enforce
complete JSON validation and live rereads, JSONL concurrency, HTTP request
encoding, authentication, statuses, content types, response bounds, timeouts,
exact event payloads, and no-retry behavior. HTTP service tests cover every
redirect status and outcome, multi-host tenancy, stale success, raw-path identity,
`GET`/`HEAD`/unsupported methods, proxy failures, source and sink failures, and
duration boundaries. An integration test assembles a path-backed JSON source, the
runtime, the Axum service, and a real JSONL sink. CI also builds and runs the
production image as its non-root user, probes health and a real redirect/event,
and verifies graceful SIGTERM shutdown.

These tests protect behavior at adapter boundaries rather than making JSON or
JSONL a prerequisite for future implementations.

## Extension points and tradeoffs

New sources implement `RedirectSource`; new event destinations implement
`RedirectEventSink`. Adapters remain independent and should preserve the domain
invariants above. A retrying sink, batching policy, or stronger durability
guarantee requires an explicit plan because it changes
latency, failure, backpressure, or shutdown semantics.

The design favors a simple bundled file adapter behind a shared runtime lifecycle
and per-record event flush over maximum throughput. It favors stale redirect
availability over surfacing refresh failures, explicit trusted-proxy configuration
over unsafe auto-detection, raw path identity over aggressive normalization, and
non-fatal event failure over sacrificing the primary redirect response.

Compactor is distributed under the Apache License, Version 2.0. Licensing does
not alter runtime boundaries, but all adapters and documentation remain part of
the same Apache 2.0 project unless separately identified.
