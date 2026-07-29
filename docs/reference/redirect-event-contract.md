# Redirect Event Contract

```rust
#[async_trait]
pub trait RedirectEventSink: Send + Sync {
    async fn emit(
        &self,
        event: &RedirectEvent,
    ) -> Result<(), RedirectEventSinkError>;
}
```

The HTTP/application layer supplies a complete sanitized event. A sink must not
inspect requests, resolve clients, invent timestamps, or determine outcomes.
Supported outcomes are `redirected`, `not_found`, `invalid_request`, and
`source_error`. Sink failure is operational and is never itself a redirect outcome.

`event_id` is a sortable ULID unique to the transaction. `redirect_id` is the
optional stable source identity and is absent for unresolved requests.
