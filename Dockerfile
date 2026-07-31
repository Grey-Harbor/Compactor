# syntax=docker/dockerfile:1
FROM rust:1.85-bookworm AS builder
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --locked --release

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install --no-install-recommends -y ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --system --gid 10001 compactor \
    && useradd --system --uid 10001 --gid compactor --home /nonexistent compactor \
    && mkdir -p /etc/compactor /var/lib/compactor \
    && chown compactor:compactor /var/lib/compactor
COPY --from=builder /build/target/release/compactor /usr/local/bin/compactor
USER 10001:10001
ENV COMPACTOR_BIND_ADDRESS=0.0.0.0:8080 \
    COMPACTOR_REDIRECTS_FILE=/etc/compactor/redirects.json \
    COMPACTOR_EVENTS_FILE=/var/lib/compactor/events.jsonl \
    COMPACTOR_REDIRECT_CACHE_TTL_SECONDS=300 \
    COMPACTOR_REDIRECT_CACHE_MAX_ENTRIES=10000
EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD ["curl", "--fail", "--silent", "http://127.0.0.1:8080/healthz"]
ENTRYPOINT ["/usr/local/bin/compactor"]
