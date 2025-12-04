# syntax=docker/dockerfile:1.6

FROM oven/bun:1.3-slim AS bun-base
WORKDIR /app

COPY package.json bun.lock tsconfig.json biome.json ./
COPY scripts ./scripts
COPY migrations ./migrations

RUN bun install --frozen-lockfile

FROM bun-base AS bun-builder
COPY frontend ./frontend
RUN bun run scripts/build.ts

FROM rust:1.91-slim-trixie AS rust-builder
WORKDIR /app

RUN apt-get update \
	&& apt-get install -y --no-install-recommends build-essential pkg-config ca-certificates git \
	&& rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY templates ./templates
COPY migrations ./migrations
COPY --from=bun-builder /app/dist ./dist
RUN cargo fetch --locked

RUN cargo build --release --locked

FROM debian:trixie-slim
WORKDIR /app

RUN apt-get update \
	&& apt-get install -y --no-install-recommends ca-certificates git openssh-client curl \
	&& rm -rf /var/lib/apt/lists/*

COPY --from=rust-builder /app/target/release/rubhub /usr/local/bin/rubhub
COPY --from=rust-builder /app/dist ./dist
COPY --from=rust-builder /app/templates ./templates

RUN mkdir -p /app/data/git /app/data/assets
VOLUME ["/app/data"]

ENV HTTP_BIND_ADDRESS=0.0.0.0 \
	HTTP_BIND_PORT=3000 \
	SSH_BIND_ADDRESS=0.0.0.0 \
	SSH_PORT=2222

EXPOSE 3000 2222

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
	CMD curl -f http://127.0.0.1:3000/ || exit 1

CMD ["rubhub"]
