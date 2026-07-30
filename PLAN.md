# Project Plan

## v0.1

- [x] Define domain contracts and validated types.
- [x] Load and validate an immutable JSON redirect source.
- [x] Append complete events to a serialized JSONL sink.
- [x] Serve redirects with bounded, proxy-aware event capture.
- [x] Package the service for non-root container operation.
- [x] Document operation, formats, architecture, and design intent.
- [x] Cover contracts, adapters, and the integrated HTTP path with tests.

## v0.1 pre-release audit

- [x] Preserve path case, trailing slashes, dot segments, repeated separators,
  and percent-encoding spelling in canonical lookup keys.
- [x] Route unsupported methods on `/healthz` through the normal invalid-request
  response and event path while keeping `GET` and `HEAD` event-free.
- [x] Exercise the JSON source, HTTP service, and JSONL sink together and close
  focused configuration, proxy, duration, and adapter-error coverage gaps.
- [x] Add a container runtime smoke test covering the non-root user, health
  endpoint, redirect/event path, and graceful `SIGTERM` shutdown.
- [x] Expand `ARCHITECTURE.md` to record system boundaries, invariants, lifecycle,
  data ownership, security model, deployment topology, and tradeoffs.
- [x] Keep `RELEASE.md` synchronized with the behavior actually verified for
  v0.1.0.

## Assumptions

- TLS terminates before Compactor; configured trusted proxies reconstruct the
  public scheme and authority.
- Source content is replaced through deployment/configuration management and takes
  effect at process restart.
- Event rotation, shipping, retention, and analysis are external.

Future work must be justified by a concrete adapter or operational requirement and
documented here before significant implementation begins.
