# Build Compactor from source

This tutorial walks through a complete local build of Compactor from a fresh
checkout.

Use it when you want to compile the service, run the verification suite, exercise
the redirect pipeline, and preview the documentation site as GitHub Pages will
serve it.

If you only want to run one redirect, use the shorter
[getting-started tutorial](./getting-started.md). For container deployment, use
the [Docker guide](../how-to/deploy-with-docker.md).

## Clone the repository

Start from a clean checkout:

```bash
git clone git@github.com:Grey-Harbor/Compactor.git
cd Compactor
```

This tutorial assumes commands run from the repository root unless a step says
otherwise.

## Install Rust 1.85

Compactor supports Rust 1.85 and newer. Install the minimum supported toolchain
with the components used by CI:

```bash
rustup toolchain install 1.85.0 \
  --profile minimal \
  --component clippy,rustfmt
```

Use `cargo +1.85.0` in the commands below when another toolchain is your default.

## Build the service

Compile the locked dependency graph:

```bash
cargo +1.85.0 build --locked
```

The debug executable is written to `target/debug/compactor`.

## Exercise the redirect pipeline

Copy the example source document:

```bash
cp examples/redirects.json redirects.json
```

Start Compactor from source:

```bash
cargo +1.85.0 run --locked
```

In another terminal, check readiness and follow the example redirect:

```bash
curl --fail http://127.0.0.1:8080/healthz
curl -i 'http://127.0.0.1:8080/project?source=build-tutorial'
```

The second response is `302 Found`. Its `Location` retains the incoming query,
and `events.jsonl` receives a sanitized record of the completed request.

Stop the service with `Ctrl-C` before continuing.

## Run the service verification

Run the same Rust checks used by the repository's **Verify** job:

```bash
cargo +1.85.0 fmt --all -- --check
cargo +1.85.0 test --locked --all-targets
cargo +1.85.0 clippy --locked --all-targets --all-features -- -D warnings
```

Validate the Compose model and production image separately:

```bash
docker compose config --quiet
docker build --tag compactor:source .
```

These commands verify the source tree without publishing an image or starting a
hosted service.

## Build the site and documentation

The GitHub Pages project uses Node.js 22. Install its locked dependencies:

```bash
node --version
npm --prefix site ci
```

Then run the same site checks as CI:

```bash
npm --prefix site run check
npm --prefix site run build
```

The build produces `site/out/`, including the marketing page, Fumadocs-rendered
Diátaxis documentation, crawler files, brand assets, and custom-domain file.

## Preview the Pages output locally

Preview the generated export rather than the Next.js development server:

```bash
npm --prefix site run preview
```

Open `http://127.0.0.1:3000` and check the homepage, `/docs/`,
`/docs/tutorials/`, and the configuration reference. This server reads
`site/out/`, so it matches the artifact GitHub Pages publishes.

## Verify the Pages artifact

Confirm that the export contains its domain, crawler, and brand files:

```bash
test -f site/out/CNAME
test -f site/out/robots.txt
test -f site/out/sitemap.xml
test -f site/out/brand/compactor-mark.svg
test -f site/out/brand/social-card.png
```

`CNAME` must contain `compactor.greyharborsoftware.com`. `robots.txt` must allow
indexing and point to the HTTPS sitemap. The sitemap must include the homepage
and every route generated from `docs/`.

The [Publish website](../../.github/workflows/pages.yml) workflow runs after
changes reach `main` and can also be started manually from GitHub Actions. It
installs the locked site dependencies, checks the TypeScript project, builds the
static export, and deploys `site/out/`.

## What a healthy build looks like

At the end of this tutorial, you should be able to:

- compile Compactor with the minimum supported Rust toolchain;
- run a configured redirect and inspect its event;
- pass the service and site verification commands;
- build the production container;
- export the site into `site/out/`; and
- preview the marketing page and Fumadocs documentation locally.

If the service checks pass but the site export fails, debug them separately. The
Rust checks cover request behavior and adapters; the site build covers
documentation rendering and GitHub Pages.

## Where to go next

- Configure a complete redirect source with the
  [JSON source guide](../how-to/configure-json-source.md).
- Review public URL behavior in the
  [normalization reference](../reference/url-normalization.md).
- Deploy the production image with the
  [Docker guide](../how-to/deploy-with-docker.md).
