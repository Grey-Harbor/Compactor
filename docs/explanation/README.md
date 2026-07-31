# Explanation

Read these pages when you need the reasoning behind Compactor's boundaries rather
than a procedure or a field-level contract.

- [Architecture](architecture.md) explains how HTTP orchestration, cached runtime
  lifecycle, and adapters remain separate.
- [Design philosophy](design-philosophy.md) describes the reliability choices
  behind startup validation and best-effort events.
- [Adapter model](adapter-model.md) shows how to add storage integrations without
  coupling them to transport or cache behavior.
- [Why this is not a URL shortener](why-not-a-url-shortener.md) helps adopters
  decide whether Compactor is the right product boundary.
