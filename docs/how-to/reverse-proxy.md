# Run Behind a Reverse Proxy

Compactor serves plain HTTP. Terminate TLS at a proxy, preserve or forward the
public authority, and configure only the proxy's actual source networks:

```sh
export COMPACTOR_TRUSTED_PROXIES='10.20.0.0/16,192.0.2.10'
```

Prefer an RFC 7239 header such as:

```text
Forwarded: for=203.0.113.8;proto=https;host=go.example.com
```

`X-Forwarded-For`, `X-Forwarded-Proto`, and `X-Forwarded-Host` are supported as a
fallback when the corresponding `Forwarded` information is absent. Compactor
ignores all forwarding headers from untrusted peers, appends the immediate peer to
the address chain, walks from right to left past trusted proxies, and rejects
malformed metadata from trusted peers. Ensure the proxy replaces client-supplied
forwarding headers instead of blindly appending to them.

The reconstructed public scheme, host, effective port, and path must match the
configured [canonical URL](../reference/url-normalization.md).
