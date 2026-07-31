# Redirect Source Contract

Implement this contract when redirect definitions need to come from something
other than the bundled JSON file. The application supplies one normalized
canonical URL and expects one of three results: a definition, no match, or an
operational failure.

```rust
#[async_trait]
pub trait RedirectSource: Send + Sync {
    async fn resolve(
        &self,
        canonical_url: &CanonicalUrl,
    ) -> Result<Option<RedirectDefinition>, RedirectSourceError>;
}
```

A successful definition contains:

| Field | Requirement |
| --- | --- |
| ID | Non-empty, opaque, stable, and owned by the source |
| Canonical URL | Equal to the normalized lookup key |
| Destination | A valid absolute URL |
| Status | One of `301`, `302`, `303`, `307`, or `308` |
| Response headers | Valid header names and values that do not override protocol-owned headers |

Interpret the return value as follows:

| Result | Application behavior |
| --- | --- |
| `Ok(Some(definition))` | Cache the definition according to runtime policy and return it |
| `Ok(None)` | Keep the key absent; a cold HTTP lookup returns `404 Not Found` |
| `Err(error)` | Keep the key absent; a cold HTTP lookup returns `500 Internal Server Error` |

Keep source errors distinct from missing records. Treating a failed backend as a
miss would turn an operational incident into incorrect public behavior. During a
background refresh, the runtime instead keeps serving the stale definition and
logs an error; the triggering HTTP event remains `redirected`.

The contract does not know about JSON, HTTP requests, event persistence, TTL,
residency, eviction, refresh, or cross-request coordination. Validate external
records while converting them into the domain types, before making them visible
to the runtime. A returned definition's canonical URL must equal the supplied key;
the runtime rejects a mismatch as a source error.

Implement finite I/O timeouts inside network-backed adapters. Graceful shutdown
waits for source work already in flight and does not impose an independent timeout.
For the bundled implementation, see the [JSON source format](json-source-format.md).
