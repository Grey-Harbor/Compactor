# Redirect Source Contract

Implement this contract when redirect definitions need to come from something
other than the bundled JSON file. The application supplies one normalized
canonical URL and expects one of three results: a definition, no match, or an
operational failure.

```rust
pub trait RedirectSource: Send + Sync {
    fn resolve(
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
| `Ok(Some(definition))` | Return the configured redirect and emit `redirected` |
| `Ok(None)` | Return `404 Not Found` and emit `not_found` |
| `Err(error)` | Return `500 Internal Server Error` and emit `source_error` |

Keep source errors distinct from missing records. Treating a failed backend as a
miss would turn an operational incident into incorrect public behavior.

The contract does not know about JSON, HTTP requests, or event persistence.
Validate external records while converting them into the domain types, before
making them visible to requests. For the bundled implementation, see the
[JSON source format](json-source-format.md).
