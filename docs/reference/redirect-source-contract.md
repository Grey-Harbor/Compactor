# Redirect Source Contract

```rust
pub trait RedirectSource: Send + Sync {
    fn resolve(
        &self,
        canonical_url: &CanonicalUrl,
    ) -> Result<Option<RedirectDefinition>, RedirectSourceError>;
}
```

A source maps one normalized canonical URL to an optional definition containing an
opaque ID, canonical URL, absolute destination, one of `301`, `302`, `303`, `307`,
or `308`, and validated response headers. `None` means not found; `Err` means the
source failed. IDs are source-owned and never derived by Compactor.

The contract does not know about JSON, HTTP requests, or event persistence.
