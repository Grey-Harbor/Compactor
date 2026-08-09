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

## Documentation quality

Documentation must be instructional first while preserving precise technical
definitions. Preserve the tone and depth established in the current `docs/` tree.

For every documentation change:

- Begin pages by telling readers when and why to use the information.
- Prefer complete, copyable examples over fragments.
- Format JSON and JSONL examples for human review. Show JSONL pretty-printed in
  documentation while clearly stating that the stored representation is one
  object per physical line.
- Define inputs, outputs, defaults, invariants, failure behavior, ownership, and
  operational limitations explicitly.
- Distinguish guaranteed behavior from recommendations and adapter-specific
  behavior.
- Include rollout, rollback, security, persistence, and observability guidance
  where operationally relevant.
- Write for human and AI adopters. Automation guidance must identify which
  transformations are safe and which business or operational decisions must not
  be inferred.
- Cross-reference canonical documents rather than duplicating contracts.
- Validate fenced JSON, internal links, the Fumadocs type check, and the static
  site export before committing documentation changes.

## Planning

Use the local-only, Git-ignored `PLAN.md` before significant features or
architectural work. Record assumptions, affected components, and implementation
strategy there. Do not commit `PLAN.md`, and do not begin a large undocumented
refactor.

## Code quality and testing

- Write readable code before optimizing and keep functions focused.
- Favor explicit types, small public APIs, and meaningful error messages.
- Avoid unnecessary dependencies and remove dead or commented-out code.
- Add focused tests with behavior changes and preserve observable behavior during
  refactors.
- Update tests, documentation, and architecture together when their contract
  changes.
- Docker-based tests and smoke checks must remove every Docker asset they create
  after completion or failure, including containers, images, networks, volumes,
  and temporary files. Never remove pre-existing or user-owned Docker assets.

## Licensing

Compactor is licensed under the Apache License, Version 2.0. Keep `LICENSE`,
package metadata, and any documentation that discusses project licensing aligned
with Apache 2.0. Do not add license or copyright headers to individual source files
unless a maintainer explicitly requests them.

## Continuous integration

Keep `.github/workflows/ci.yml` aligned with the local verification commands and
the minimum supported Rust version. CI must remain read-only. Container publishing
is restricted to stable semantic version tags through
`.github/workflows/release.yml`; do not add branch-based publishing or service
deployment without explicit maintainer authorization.

`RELEASE.md` is the canonical summary and annotated-tag description for the
current release. Keep its version aligned with `Cargo.toml`, update its summary
whenever release-facing behavior changes, and create release tags with
`git tag -a --cleanup=verbatim v<version> -F RELEASE.md`. Release tags and their
immutable container-image tags always begin with `v`; `latest` is the only moving
image alias. Verbatim cleanup is required so Git preserves Markdown headings in
the tag description. Never rewrite a published tag; prepare `RELEASE.md` for the
next version instead.

## Git standards

Treat history as an engineering artifact. All updates must be developed on a
working branch; never make new changes directly on `main`. Start from an up-to-date
`main`, create a short descriptive branch such as `feat/header-policy`,
`fix/proxy-chain`, or `docs/deployment`, and do not prefix branch names with
`codex/` or another agent name.

Use this workflow for every change:

1. Create or switch to the working branch before editing.
2. Make one logical change per commit and validate it locally.
3. Push the working branch to `origin`.
4. Open a pull request targeting `main` with the change summary and verification
   evidence.
5. Merge only after required CI and review are complete.
6. Update local `main` from the merged remote before starting other work.

Do not bypass the pull request by pushing commits directly to `main`. Avoid mixing
refactors with feature work, and never commit generated output or temporary
debugging changes unless explicitly required.

Every commit uses Conventional Commits:

```text
<type>(<scope>): <description>
```

Use imperative mood and keep the first line under 72 characters. Preferred types
are `feat`, `fix`, `docs`, `refactor`, `test`, `chore`, `ci`, `build`, and `perf`.
Add a `BREAKING CHANGE` footer when applicable.

## Release tags

Create a release tag only after the release pull request is merged and local
`main` is updated to that exact merged commit. Confirm `Cargo.toml` and
`RELEASE.md` name the same version, rerun the release checks, then create and push
the annotated tag:

```sh
git tag -a --cleanup=verbatim v<version> -F RELEASE.md
git push origin v<version>
```

Never tag an unmerged working branch, move or recreate a published tag, or publish
a release image from a branch. The tag push is the only action that triggers the
container release workflow, which publishes the exact `v<version>` image tag and
the moving `latest` alias.

## Review mindset

Work as though every change receives peer review, including work on the default
branch. Each modification must be intentional, documented, tested, and internally
consistent. Leave the codebase clearer than you found it, and keep implementation
and architecture documentation synchronized.
