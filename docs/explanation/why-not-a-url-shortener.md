# Why Compactor Is Not a URL Shortener

A URL shortener usually owns alias creation, users, campaigns, editing, analytics,
and a management surface. Compactor owns none of them. It resolves an already
configured canonical URL and emits a transaction event.

That narrow responsibility lets configuration management choose redirects and
external data systems choose retention and analysis. Compactor stays replaceable,
auditable, and dependable on the request path instead of becoming another
administrative application.
