# Publish a Tagged Container Release

Container publication is restricted to stable semantic version tags. Before
releasing, ensure `Cargo.toml` contains the intended version and all changes are
merged into `main`.

Create and push an annotated tag:

```sh
git tag -a v0.1.0 -m "Release v0.1.0"
git push origin v0.1.0
```

The release workflow runs the complete CI gate before publishing Linux AMD64 and
ARM64 images to GitHub Container Registry. For `v0.1.0`, it publishes:

```text
ghcr.io/grey-harbor/compactor:0.1.0
ghcr.io/grey-harbor/compactor:0.1
ghcr.io/grey-harbor/compactor:0
ghcr.io/grey-harbor/compactor:latest
```

Tags that do not exactly match `vMAJOR.MINOR.PATCH` fail validation. Branch pushes,
pull requests, and manual CI runs never publish an image. The workflow publishes a
container package; it does not deploy a running service.
