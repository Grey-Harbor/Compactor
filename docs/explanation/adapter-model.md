# Adapter Model

Redirect definitions enter through `RedirectSource`; completed outcomes leave
through `RedirectEventSink`. The contracts are independent, so JSON source plus
JSONL events is only the first composition.

When evaluating a new adapter, first decide which side of the boundary it serves:

- a source answers a canonical-URL lookup with a validated redirect definition;
- a sink receives a complete, sanitized event after the response is chosen.

Translate the external representation into existing domain types at the adapter
edge. Validate at the earliest reliable point, preserve the distinction between
"not found" and "backend failed," and do not expose storage-specific concepts to
the application layer.

Source and sink implementations may have different availability and consistency
needs. A source failure changes the response because Compactor cannot safely invent
a redirect. A sink failure is best-effort telemetry and does not change a response
that has already been selected.

Add contract surface only when a real integration requires it. That discipline
keeps adapters replaceable and prevents one backend's capabilities from becoming
requirements for every other backend. See the
[source](../reference/redirect-source-contract.md) and
[event](../reference/redirect-event-contract.md) contracts before implementing an
adapter.
