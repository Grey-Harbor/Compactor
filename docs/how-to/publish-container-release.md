# Publish a Tagged Container Release

Container publication is restricted to stable semantic version tags. `RELEASE.md`
is the canonical release summary and annotated-tag description. Before releasing:

1. Update its version heading and summary.
2. Set the same version in `Cargo.toml`.
3. Commit those changes and merge them into `main`.

Create the annotated tag directly from the release summary, then push it:

```sh
git tag -a --cleanup=verbatim v0.1.0 -F RELEASE.md
git push origin v0.1.0
```

The `--cleanup=verbatim` option preserves Markdown headings and blank lines.
Without it, Git treats heading lines beginning with `#` as comments, causing the
tag description validation to fail.

The release workflow runs the complete CI gate before publishing Linux AMD64 and
ARM64 images to GitHub Container Registry. For `v0.1.0`, it publishes:

```text
ghcr.io/grey-harbor/compactor:0.1.0
ghcr.io/grey-harbor/compactor:0.1
ghcr.io/grey-harbor/compactor:0
ghcr.io/grey-harbor/compactor:latest
```

Tags fail validation unless they:

- exactly match `vMAJOR.MINOR.PATCH`;
- are annotated;
- match the package version and `RELEASE.md` heading; and
- use the complete current `RELEASE.md` as their description.

Branch pushes, pull requests, and manual CI runs never publish an image. The
workflow publishes a container package; it does not deploy a running service.
