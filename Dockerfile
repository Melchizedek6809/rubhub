# syntax=docker/dockerfile:1.6

##################
# 1. Frontend build
##################

FROM node:24-alpine as frontend
WORKDIR /app

COPY package.json package-lock.json tsconfig.json vite.config.ts ./
RUN npm ci
COPY frontend ./frontend
RUN npm run build


##################
# 2. Rust builder
##################

FROM rust:1.91-alpine3.22 AS rust-builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY templates ./templates
COPY crates ./crates
COPY public ./public
COPY --from=frontend /app/dist ./dist
RUN cargo build --release --locked


##################
# 3. Final image
##################

FROM alpine:3.22
WORKDIR /app

RUN apk add --no-cache git curl openssh
RUN addgroup -S rubhub && adduser -S -G rubhub -h /app rubhub

COPY --from=rust-builder /app/target/release/rubhub /usr/local/bin/rubhub
RUN mkdir -p /app/data && chown -R rubhub:rubhub /app
VOLUME ["/app/data"]

ENV HTTP_BIND_ADDRESS=0.0.0.0 \
	HTTP_BIND_PORT=3000 \
	SSH_BIND_ADDRESS=0.0.0.0 \
	SSH_PORT=2222

EXPOSE 3000 2222

USER rubhub

HEALTHCHECK --interval=60s --timeout=5s --start-period=10s --retries=3 \
	CMD curl -f http://127.0.0.1:3000/ || exit 1

CMD ["rubhub"]
