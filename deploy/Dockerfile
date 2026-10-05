FROM node:22-bookworm-slim AS web-builder
WORKDIR /workspace/web
COPY web/package.json web/package-lock.json ./
RUN npm ci
COPY web/ ./
ARG VITE_MAP_STYLE_URL
ARG VITE_MAPTILER_KEY
ENV VITE_MAP_STYLE_URL=$VITE_MAP_STYLE_URL
ENV VITE_MAPTILER_KEY=$VITE_MAPTILER_KEY
RUN npm run build

FROM rust:1.98-bookworm AS server-builder
WORKDIR /workspace/server
COPY server/Cargo.toml server/Cargo.lock ./
COPY server/src ./src
COPY server/migrations ./migrations
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=server-builder /workspace/server/target/release/lifetrail-server /usr/local/bin/lifetrail-server
COPY --from=web-builder /workspace/web/dist /app/web
EXPOSE 8080
ENTRYPOINT ["lifetrail-server"]
CMD ["serve"]
