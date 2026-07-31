![Compactor — Make the redirect. Keep the machinery small.](site/public/brand/social-card.png)

# Compactor

[![CI](https://github.com/Grey-Harbor/Compactor/actions/workflows/ci.yml/badge.svg)](https://github.com/Grey-Harbor/Compactor/actions/workflows/ci.yml)

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

The local example is intentionally minimal. Before routing public traffic, assign
ownership for proxy trust, event retention, source rollout, and rollback using the
[production readiness guide](docs/how-to/prepare-for-production.md).

## Documentation

- [Compactor website](https://compactor.greyharborsoftware.com)
- [Documentation index](docs/README.md)
- [Getting started](docs/tutorials/getting-started.md)
- [Building from source](docs/tutorials/building-from-source.md)
- [Configuration reference](docs/reference/configuration.md)
- [Architecture](ARCHITECTURE.md)
- [Current release summary](RELEASE.md)
- [Deployment with Docker](docs/how-to/deploy-with-docker.md)
- [Production readiness](docs/how-to/prepare-for-production.md)
- [Why Compactor is not a URL shortener](docs/explanation/why-not-a-url-shortener.md)

Compactor v0.1 requires Rust 1.85 or newer.

The website publishes this repository's Markdown documentation through Fumadocs.
The files under `docs/` remain the source of truth.

From the repository root, use `npm run site:check`, `npm run site:build`, and
`npm run site:preview` to verify, export, and preview the website.

## Continuous integration

Pull requests, pushes to `main`, and manual workflow runs verify formatting, tests,
strict Clippy checks, the Compose model, the production container build, and a
non-root runtime smoke test covering health, redirects, events, and graceful
shutdown. The CI workflow does not publish artifacts.

Pushing a stable semantic version tag such as `v0.1.0` runs the complete CI gate,
then publishes Linux AMD64 and ARM64 images with provenance and SBOM attestations
to GitHub Container Registry:

```sh
docker pull ghcr.io/grey-harbor/compactor:0.1.0
```

Release images are also tagged with their minor, major, and `latest` aliases. No
workflow deploys a running Compactor service.

## License

Compactor is licensed under the
[Apache License, Version 2.0](https://www.apache.org/licenses/LICENSE-2.0).
See [LICENSE](LICENSE) for the complete terms.
