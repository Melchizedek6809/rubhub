# syntax=docker/dockerfile:1.6

FROM oven/bun:1.3-alpine AS bun
WORKDIR /app

COPY package.json bun.lock tsconfig.json biome.json ./
RUN bun install --frozen-lockfile
COPY scripts ./scripts
COPY frontend ./frontend
RUN bun run scripts/build.ts

FROM rust:1.91-alpine3.22 AS rust-builder
WORKDIR /app

COPY Cargo.toml Cargo.lock dummy.rs ./
RUN cargo build --release --locked --bin dummy-to-cache-dependencies

COPY src ./src
COPY templates ./templates
COPY --from=bun /app/dist ./dist
RUN cargo build --release --locked

FROM alpine:3.22
WORKDIR /app

RUN apk add --no-cache git curl openssh

COPY --from=rust-builder /app/target/release/rubhub /usr/local/bin/rubhub

RUN mkdir -p /app/data/git /app/data/assets
VOLUME ["/app/data"]

ENV HTTP_BIND_ADDRESS=0.0.0.0 \
	HTTP_BIND_PORT=3000 \
	SSH_BIND_ADDRESS=0.0.0.0 \
	SSH_PORT=2222

EXPOSE 3000 2222

HEALTHCHECK --interval=60s --timeout=5s --start-period=10s --retries=3 \
	CMD curl -f http://127.0.0.1:3000/ || exit 1

CMD ["rubhub"]
