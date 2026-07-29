# Design Philosophy

Compactor treats redirection as a small infrastructure primitive. Static startup
validation makes configuration failures loud and runtime behavior predictable.
Opaque IDs preserve identity independently of URLs. Explicit constrained types
make invalid statuses and protocol-owned headers difficult to represent.

The redirect response is primary; event storage is secondary. That boundary keeps
an unavailable analytics pipeline from breaking valid user traffic while still
making the failure visible to operators.
