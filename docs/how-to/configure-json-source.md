# Configure the JSON Redirect Source

Use this guide to create or replace the complete set of redirects loaded by one
Compactor process.

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
header, and duplicate validation during startup.

## Point Compactor at the file

```sh
export COMPACTOR_REDIRECTS_FILE=/etc/compactor/redirects.json
cargo run --release
```

Confirm `/healthz` succeeds, then exercise at least one known redirect using the
public `Host` value expected in `canonical_url`.

## Replace active configuration

Compactor reads the entire file once during startup. To make a change:

1. Generate and review a complete replacement document.
2. Validate it in a separate Compactor process or staging instance.
3. Keep the current document available for rollback.
4. Install the replacement through configuration management.
5. Restart Compactor and wait for `/healthz` before shifting traffic.
6. Exercise a changed redirect and inspect its event.

If any definition is invalid, startup fails and no traffic is accepted. Compactor
never serves the valid subset of a partially invalid document. Replacing the file
without restarting does not alter the in-memory source.

For automated changes, require explicit values for destinations, status codes,
headers, and public hosts. Formatting or sorting a reviewed document is safe;
inventing redirect policy is not.
