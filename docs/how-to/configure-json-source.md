# Configure the JSON Redirect Source

Use this guide to create, activate, and roll back the complete redirect document
resolved by one Compactor process.

## Create the document

Start with a readable JSON document:

```json
{
  "redirects": [
    {
      "id": "docs-home",
      "canonical_url": "https://go.example.com/docs",
      "redirect_url": "https://docs.example.com/current/",
      "status_code": 308,
      "response_headers": {
        "Cache-Control": "public, max-age=300"
      }
    }
  ]
}
```

Use a stable, non-empty `id` that survives URL changes. Set `canonical_url` to the
public URL the client requests, not the internal listener address. See the
[JSON source format](../reference/json-source-format.md) for every field and the
[URL normalization reference](../reference/url-normalization.md) before generating
paths programmatically.

Check JSON syntax when `jq` is available:

```sh
jq empty /etc/compactor/redirects.json
```

This checks syntax only. Compactor performs the authoritative field, URL, status,
header, and duplicate validation during startup and every authoritative lookup.

## Point Compactor at the file

```sh
export COMPACTOR_REDIRECTS_FILE=/etc/compactor/redirects.json
cargo run --release
```

Confirm `/healthz` succeeds, then exercise at least one known redirect using the
public `Host` value expected in `canonical_url`.

## Replace active configuration

Compactor validates the source at startup, then rereads it when the runtime has a
cold miss or refreshes a stale redirect. To make a change without restarting:

1. Generate and review a complete replacement document in a separate path.
2. Validate it in a staging instance or with the same Compactor version.
3. Keep the current document available for rollback.
4. Atomically rename the replacement over the configured path on the same
   filesystem. Do not truncate and rewrite the active file in place.
5. Exercise a changed redirect after its configured cache TTL and inspect the
   event. A newly added key is available on its first lookup.

An updated definition remains cached until its TTL expires. The first request at
expiry receives the old redirect immediately and starts a background refresh;
later requests receive the replacement after refresh succeeds. A deleted
definition is similarly served once after expiry, then removed.

If any definition in a replacement is invalid, cold keys return `500` with a
`source_error` event. Existing resident redirects remain available as stale data;
refresh failures are logged and retried no more than once per key every 30 seconds.
Compactor never serves the valid subset of a partially invalid document.

## Roll back a source change

Atomically restore the previous complete document at the configured path. No
process restart is required. Cold keys recover on their next lookup; stale entries
recover on the first eligible refresh. `/healthz` reports process health and does
not validate a replacement after startup, so verify a known redirect and monitor
source-error logs during rollout and rollback.

For automated changes, require explicit values for destinations, status codes,
headers, and public hosts. Formatting or sorting a reviewed document is safe;
inventing redirect policy is not.
