FROM rust:1.96-slim-bookworm AS builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends build-essential pkg-config libssl-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --locked --bin telitairos-bot

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --no-create-home --shell /usr/sbin/nologin bot

COPY --from=builder /app/target/release/telitairos-bot /usr/local/bin/telitairos-bot

ENV RUST_LOG=info
USER bot
STOPSIGNAL SIGINT
CMD ["/usr/local/bin/telitairos-bot"]
