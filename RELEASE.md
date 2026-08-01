# Compactor v0.2.0

Compactor v0.2.0 introduces a source-independent cached redirect runtime plus
provider-neutral HTTP source and event sink adapters. Every source resolves
authoritative state through the same bounded, stale-while-revalidate lifecycle.

## Highlights

- Adds a bounded positive-result cache with configurable TTL, LRU eviction,
  same-key cold single-flight, and one coordinated refresh per stale redirect.
- Serves stale redirects without refresh latency, removes authoritative deletions
  after one final stale response, and retains stale data through source failures
  with a 30-second retry cooldown.
- Makes `RedirectSource` asynchronous and keeps cache policy entirely in the
  runtime so future file, HTTP, database, and custom adapters share one lifecycle.
- Changes the bundled JSON source to validate at startup and reread complete,
  atomically replaced documents during authoritative resolution.
- Drains background redirect refreshes during graceful shutdown while preserving
  the existing redirect, proxy, privacy-aware event, and JSONL sink behavior.
- Adds bounded one-key HTTP source lookups and one-POST event delivery with shared
  pooled rustls transport, bearer or static-header authentication, strict response
  validation, and no adapter retries.
- Selects source and sink adapters independently, includes a runnable local mock,
  and keeps health probes independent from remote services.
- Includes a non-root production container for Linux AMD64 and ARM64.
- Provides focused contract and adapter tests, JSON and HTTP integration coverage,
  and non-root container smoke tests for file and remote adapter compositions.

## Container

Tagged releases are published as `ghcr.io/grey-harbor/compactor`. Version
`v0.2.0` produces the `0.2.0`, `0.2`, `0`, and `latest` image tags.

## License

Compactor is licensed under the Apache License, Version 2.0.
