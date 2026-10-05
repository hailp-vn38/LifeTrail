# LifeTrail API server image (Rust/Axum only).
#
# The web SPA is a separate `web` service (deploy/web.Dockerfile). This image
# contains no Node.js and no built web assets; it serves the API and health
# endpoints on the internal Compose network.
FROM rust:1.98-bookworm AS builder
WORKDIR /workspace/server
COPY server/Cargo.toml server/Cargo.lock ./
COPY server/src ./src
COPY server/migrations ./migrations
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /workspace/server/target/release/lifetrail-server /usr/local/bin/lifetrail-server
EXPOSE 8080
ENTRYPOINT ["lifetrail-server"]
CMD ["serve"]
