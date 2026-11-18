# syntax=docker/dockerfile:1.7

# ---- Frontend build -------------------------------------------------------
FROM node:22-bookworm-slim AS frontend

ENV PNPM_HOME=/pnpm
ENV PATH=$PNPM_HOME:$PATH
RUN corepack enable

WORKDIR /app/frontend
COPY frontend/package.json frontend/pnpm-lock.yaml ./
RUN --mount=type=cache,target=/root/.local/share/pnpm/store \
    pnpm install --frozen-lockfile
COPY frontend/ ./
RUN pnpm build

# ---- Rust build -----------------------------------------------------------
FROM rust:1-bookworm AS builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY migration ./migration
COPY src ./src
COPY config ./config
COPY .cargo ./.cargo
COPY --from=frontend /app/frontend/dist ./frontend/dist

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release --bin braindrop-cli \
    && cp target/release/braindrop-cli /usr/local/bin/braindrop-cli

# ---- Runtime --------------------------------------------------------------
FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --create-home --home-dir /app braindrop \
    && mkdir -p /data \
    && chown braindrop:braindrop /data

WORKDIR /app
COPY --from=builder /usr/local/bin/braindrop-cli /usr/local/bin/braindrop-cli
COPY config ./config
COPY --from=frontend /app/frontend/dist ./frontend/dist

ENV LOCO_ENV=production \
    PORT=5150 \
    BINDING=0.0.0.0

USER braindrop
EXPOSE 5150

HEALTHCHECK --interval=10s --timeout=3s --start-period=10s --retries=6 \
    CMD curl -fsS http://127.0.0.1:5150/_health || exit 1

ENTRYPOINT ["braindrop-cli"]
CMD ["start"]
