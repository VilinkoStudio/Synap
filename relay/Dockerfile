FROM rust:1.89-bookworm AS builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY cli ./cli
COPY corenet ./corenet
COPY core ./core
COPY coreffi ./coreffi
COPY desktop_linux ./desktop_linux
COPY relay ./relay
COPY web ./web
COPY xtask ./xtask

RUN cargo build --locked --release --package relay

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/relay /usr/local/bin/relay

EXPOSE 8787

ENV SYNAP_RELAY_LISTEN=0.0.0.0:8787
ENV SYNAP_RELAY_SERVER_NAME=synap-relay
ENV SYNAP_RELAY_REDIS_URL=redis://127.0.0.1:6379/
ENV SYNAP_RELAY_EMBEDDED_REDIS=enabled
ENV SYNAP_RELAY_REDIS_LISTEN=127.0.0.1:6379
ENV SYNAP_RELAY_NO_KEY=true

ENTRYPOINT ["/usr/local/bin/relay"]
CMD ["serve"]
