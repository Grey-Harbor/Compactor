# Publish a Tagged Container Release

Container publication is restricted to stable semantic version tags. `RELEASE.md`
is the canonical release summary and annotated-tag description.

Only release after the release pull request is merged and local `main` points to
that exact merge commit.

## Prepare the release

1. Update the version in `Cargo.toml`.
2. Update the `RELEASE.md` heading to the matching `vMAJOR.MINOR.PATCH` value.
3. Rewrite the release summary for adopter-visible behavior, compatibility, image
   details, and licensing.
4. Run the complete local verification described in
   [Build Compactor from source](../tutorials/building-from-source.md).
5. Commit, push, review, and merge the release change.
6. Switch to `main` and fast-forward from `origin`.

Confirm the versions agree before tagging:

```sh
sed -n 's/^version = "\([^"]*\)"$/\1/p' Cargo.toml | head -n 1
sed -n '1p' RELEASE.md
```

## Create and push the tag

Create the annotated tag directly from the release summary, then push it:

```sh
git tag -a --cleanup=verbatim v0.1.0 -F RELEASE.md
git push origin v0.1.0
```

The `--cleanup=verbatim` option preserves Markdown headings and blank lines.
Without it, Git treats heading lines beginning with `#` as comments, causing the
tag description validation to fail.

Never move, recreate, or force-push a published release tag. Prepare the next
version when a correction is required.

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

After the workflow succeeds, pull one immutable version tag, inspect its platform
manifest, and run the documented health and redirect smoke test before announcing
the release.
