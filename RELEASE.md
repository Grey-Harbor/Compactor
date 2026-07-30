# Compactor v0.1.0

Compactor v0.1.0 is the initial reference release of Grey Harbor's lightweight,
adapter-driven URL redirection service.

## Highlights

- Resolves normalized, multi-host canonical URLs through an independent redirect
  source contract.
- Ships with a startup-validated JSON source and append-only JSONL event sink.
- Supports `301`, `302`, `303`, `307`, and `308` redirects, query preservation,
  trusted proxies, bounded event capture, and graceful shutdown.
- Includes a non-root production container for Linux AMD64 and ARM64.
- Provides focused contract, adapter, HTTP integration, and container tests.

## Container

Tagged releases are published as `ghcr.io/grey-harbor/compactor`. Version
`v0.1.0` produces the `0.1.0`, `0.1`, `0`, and `latest` image tags.

## License

Compactor is licensed under the Apache License, Version 2.0.
