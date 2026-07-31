# Compactor v0.2.0

Compactor v0.2.0 introduces a source-independent cached redirect runtime. Every
source now resolves authoritative state through the same bounded,
stale-while-revalidate lifecycle.

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
- Includes a non-root production container for Linux AMD64 and ARM64.
- Provides focused contract and adapter tests, full JSON-to-HTTP-to-JSONL
  integration coverage, and a non-root container runtime smoke test.

## Container

Tagged releases are published as `ghcr.io/grey-harbor/compactor`. Version
`v0.2.0` produces the `0.2.0`, `0.2`, `0`, and `latest` image tags.

## License

Compactor is licensed under the Apache License, Version 2.0.
