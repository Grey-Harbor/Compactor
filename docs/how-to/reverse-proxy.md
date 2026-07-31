# Run Behind a Reverse Proxy

Compactor serves plain HTTP. Use a reverse proxy to terminate TLS and provide the
public scheme, host, and client chain used for lookup and events.

## Configure the proxy to replace metadata

The proxy must discard client-supplied forwarding headers. This Nginx example
sets each fallback header from values Nginx observed directly:

```nginx
location / {
    proxy_pass http://compactor:8080;
    proxy_set_header Host $http_host;
    proxy_set_header X-Forwarded-Proto $scheme;
    proxy_set_header X-Forwarded-Host $http_host;
    proxy_set_header X-Forwarded-For $remote_addr;
}
```

Do not use a configuration that blindly appends an untrusted incoming
`X-Forwarded-For` value.

## Trust only the immediate proxy

Configure the actual source addresses from which Compactor receives proxy
connections:

```sh
export COMPACTOR_TRUSTED_PROXIES='10.20.0.0/16,192.0.2.10'
```

An exact IP is safer than a broad network when the deployment provides stable
addresses. Leave the value empty for direct deployments.

RFC 7239 `Forwarded` takes precedence when present:

```text
Forwarded: for=203.0.113.8;proto=https;host=go.example.com
```

`X-Forwarded-For`, `X-Forwarded-Proto`, and `X-Forwarded-Host` fill only information
missing from `Forwarded`.

## Verify the trust boundary

Test through the proxy with a public URL that exactly matches a configured
canonical URL. Then test a direct connection to Compactor containing forged
forwarding headers; the headers must be ignored. Finally, send malformed metadata
from a trusted proxy and confirm the request is rejected with `400` rather than
looked up under an ambiguous identity.

Compactor appends the immediate peer to the address chain and walks from right to
left past trusted proxies. The nearest remaining address becomes the client
address when recording is enabled.

The reconstructed public scheme, host, effective port, and path must match the
configured [canonical URL](../reference/url-normalization.md).
