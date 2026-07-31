# Why Compactor Is Not a URL Shortener

A URL shortener usually owns alias creation, users, campaigns, editing, analytics,
and a management surface. Compactor owns none of them. It resolves an already
configured canonical URL and emits a transaction event.

That narrow responsibility lets configuration management choose redirects and
external data systems choose retention and analysis. Compactor stays replaceable,
auditable, and dependable on the request path instead of becoming another
administrative application.

Choose Compactor when redirects are infrastructure-as-data: reviewed outside the
process, exposed through a source adapter, and deployed through an existing
delivery system. The bundled file source can adopt atomically installed changes
through its runtime cache without becoming a mutation API. It
is especially suitable for domain migrations, retired routes, stable aliases, and
small edge or origin services where deterministic behavior matters more than an
editing interface.

Choose a URL-shortening or link-management product when people need to create
links interactively, edit destinations without a deployment, manage tenants or
permissions, measure campaigns, or rely on a built-in analytics control plane.
Those are valuable capabilities, but they belong to a different product boundary.

For a concrete adoption checklist, continue with
[Prepare Compactor for production](../how-to/prepare-for-production.md).
