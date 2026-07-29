# Adapter Model

Redirect definitions enter through `RedirectSource`; completed outcomes leave
through `RedirectEventSink`. The contracts are independent, so JSON source plus
JSONL events is only the first composition.

A future adapter should translate its external representation into existing domain
types, validate at the earliest reliable boundary, and avoid teaching the domain
about its storage technology. New requirements should arise from a real adapter,
not speculative generalization. See the [source](../reference/redirect-source-contract.md)
and [event](../reference/redirect-event-contract.md) contracts.
