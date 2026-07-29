# Project Plan

## v0.1

- [x] Define domain contracts and validated types.
- [x] Load and validate an immutable JSON redirect source.
- [x] append complete events to a serialized JSONL sink.
- [x] Serve redirects with bounded, proxy-aware event capture.
- [x] Package the service for non-root container operation.
- [x] Document operation, formats, architecture, and design intent.
- [x] Cover contracts, adapters, and the integrated HTTP path with tests.

## Assumptions

- TLS terminates before Compactor; configured trusted proxies reconstruct the
  public scheme and authority.
- Source content is replaced through deployment/configuration management and takes
  effect at process restart.
- Event rotation, shipping, retention, and analysis are external.

Future work must be justified by a concrete adapter or operational requirement and
documented here before significant implementation begins.
