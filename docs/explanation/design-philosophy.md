# Design Philosophy

Compactor treats redirection as a small infrastructure primitive. Fail-fast
source validation makes initial configuration failures loud, while a shared
runtime cache gives every source the same predictable lifecycle. Opaque IDs
preserve identity independently of URLs. Explicit constrained types make invalid
statuses and protocol-owned headers difficult to represent.

These choices favor operations that are easy to reason about:

- the source is validated before traffic and on every authoritative resolution;
- the runtime, rather than adapters, owns freshness, residency, and concurrency;
- stale redirects remain fast and available while one refresh runs;
- requests perform deterministic lookup instead of modifying configuration;
- source-owned IDs keep analytics identity stable when URLs change;
- bounded metadata capture makes privacy and storage costs explicit; and
- health checks avoid generating business events.

The redirect response is primary; event storage is secondary. That boundary keeps
an unavailable analytics pipeline from breaking valid user traffic while still
making the failure visible to operators. It also means the bundled JSONL sink is
not a durable queue: adopters that require stronger telemetry guarantees should
implement or place a durable system downstream of the event boundary.

The same restraint applies to features. Compactor does not add a control plane,
authentication, campaign analytics, or mutable redirect APIs because those would
change its operational role. Keeping those concerns external makes the redirect
path smaller, more auditable, and easier to replace.
