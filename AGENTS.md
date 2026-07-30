# Engineering Guidance for Coding Agents

## Project philosophy

Prefer simplicity, explicitness, and maintainability over clever abstractions.
Keep modules focused, responsibilities well defined, and architectural boundaries
visible. Favor composition over inheritance. Do not add speculative features,
adapters, or abstraction layers without a demonstrated requirement.

## Architecture

The repository must contain a current, detailed `ARCHITECTURE.md`. Update it
whenever a decision changes the system boundaries, contracts, request lifecycle,
data flow, extension points, or meaningful tradeoffs. Explain why decisions were
made, not merely where files live. Documentation is part of the implementation.

## Documentation

Follow Diátaxis. Update an existing document before creating a new one, avoid
duplication, cross-reference related material, and keep tutorials, how-to guides,
reference, and explanation separate. Keep the repository approachable to new
contributors.

## Planning

Update `PLAN.md` before significant features or architectural work. Record
assumptions, affected components, and implementation strategy. Do not begin a
large undocumented refactor.

## Code quality and testing

- Write readable code before optimizing and keep functions focused.
- Favor explicit types, small public APIs, and meaningful error messages.
- Avoid unnecessary dependencies and remove dead or commented-out code.
- Add focused tests with behavior changes and preserve observable behavior during
  refactors.
- Update tests, documentation, and architecture together when their contract
  changes.

## Licensing

Compactor is licensed under the Apache License, Version 2.0. Keep `LICENSE`,
package metadata, and any documentation that discusses project licensing aligned
with Apache 2.0. Do not add license or copyright headers to individual source files
unless a maintainer explicitly requests them.

## Continuous integration

Keep `.github/workflows/ci.yml` aligned with the local verification commands and
the minimum supported Rust version. CI must remain read-only and deployment-free
until maintainers explicitly define a release destination and authorize CD.

## Git standards

Treat history as an engineering artifact. Do not prefix branch names with
`codex/` or another agent name. Make one logical change per commit, avoid mixing
refactors with feature work, and never commit generated output or temporary
debugging changes unless explicitly required.

Every commit uses Conventional Commits:

```text
<type>(<scope>): <description>
```

Use imperative mood and keep the first line under 72 characters. Preferred types
are `feat`, `fix`, `docs`, `refactor`, `test`, `chore`, `ci`, `build`, and `perf`.
Add a `BREAKING CHANGE` footer when applicable.

## Review mindset

Work as though every change receives peer review, including work on the default
branch. Each modification must be intentional, documented, tested, and internally
consistent. Leave the codebase clearer than you found it, and keep implementation
and architecture documentation synchronized.
