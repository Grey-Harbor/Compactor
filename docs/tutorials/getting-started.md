# Getting Started

This tutorial runs Compactor locally, follows one configured redirect, and reads
the event that describes the request. It takes about five minutes.

You need:

- a checkout of this repository;
- Rust 1.85 or newer; and
- `curl`.

Run every command from the repository root.

## 1. Prepare the source

Copy the maintained example to the default runtime location:

```sh
cp examples/redirects.json redirects.json
```

Open `redirects.json` and identify the four required values: `id`,
`canonical_url`, `redirect_url`, and `status_code`. The example maps
`http://localhost:8080/project` to
`https://example.com/projects/compactor` with a `302` response.

The root copy is intentionally ignored by Git. Edit it freely while learning;
keep the reusable example under `examples/` unchanged.

## 2. Start Compactor

```sh
cargo run
```

Startup succeeds only after the complete source and event destination validate.
Wait for the log message reporting that Compactor is listening on
`0.0.0.0:8080`.

In another terminal, verify readiness:

```sh
curl --fail http://127.0.0.1:8080/healthz
```

The response body is `ok`. Health requests do not create events.

## 3. Follow the redirect

```sh
curl -i 'http://localhost:8080/project?source=tutorial'
```

Confirm the response is `302 Found` and its `Location` is:

```text
https://example.com/projects/compactor?source=tutorial
```

The incoming query is appended to the configured destination but does not affect
source lookup.

## 4. Read the event

Each request appends one compact JSON object to `events.jsonl`. Format the newest
record for inspection when `jq` is available:

```sh
tail -n 1 events.jsonl | jq .
```

Check these fields:

- `outcome` is `redirected`;
- `redirect_id` is `project-home`;
- `request.path` is `/project`;
- `request.query` is `source=tutorial`; and
- `response.status_code` is `302`.

Stop Compactor with `Ctrl-C`. A graceful shutdown logs that the service stopped.
The runtime finishes any redirect refresh already in flight before that final log.

## What you learned

You supplied a startup-validated redirect source, resolved a query-free canonical
URL through the runtime cache, returned a redirect with the query appended, and
emitted a bounded event. Valid atomic source replacements can take effect later
without restarting, according to the cache TTL.

Next, learn how to [configure redirects](../how-to/configure-json-source.md) and
review the [normalization rules](../reference/url-normalization.md). To use the
container instead of Cargo, follow [Deploy with Docker](../how-to/deploy-with-docker.md).
