# Compactor

**Make the redirect. Keep the machinery small.**

Compactor is a lightweight, adapter-driven URL redirection service. Redirect
definitions are supplied through a source adapter, while request outcomes are
emitted through an independent event adapter. The reference implementation uses
JSON for redirect definitions and JSON Lines for request events.

Compactor is infrastructure, not a link-management product. It has no dashboard,
accounts, campaigns, analytics engine, or mutation API. Configuration management
and event analysis stay in the systems built for those jobs.

```text
request → canonical URL → redirect source → response → event sink
```

## Start locally

```sh
cp examples/redirects.json redirects.json
cargo run
curl -i http://localhost:8080/project?from=readme
```

Events appear in `events.jsonl`. For a containerized start:

```sh
docker compose up --build
```

## Documentation

- [Documentation index](docs/README.md)
- [Getting started](docs/tutorials/getting-started.md)
- [Configuration reference](docs/reference/configuration.md)
- [Architecture](ARCHITECTURE.md)
- [Deployment with Docker](docs/how-to/deploy-with-docker.md)
- [Why Compactor is not a URL shortener](docs/explanation/why-not-a-url-shortener.md)

Compactor v0.1 requires Rust 1.85 or newer.
