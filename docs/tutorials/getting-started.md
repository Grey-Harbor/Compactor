# Getting Started

This tutorial runs one redirect and inspects its event.

## 1. Prepare the source

```sh
cp examples/redirects.json redirects.json
```

The example maps `http://localhost:8080/project` to a destination URL.

## 2. Start Compactor

With Rust 1.85 or newer:

```sh
cargo run
```

Or use the packaged service:

```sh
docker compose up --build
```

## 3. Follow the redirect

```sh
curl -i 'http://localhost:8080/project?source=tutorial'
```

The response is `302 Found`; its `Location` includes the incoming query. Read the
new line in `events.jsonl` (or the Compose volume) to see the sanitized request and
response record.

Next, learn how to [configure redirects](../how-to/configure-json-source.md) and
review the [normalization rules](../reference/url-normalization.md).
