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

# The app uses rustls for outbound TLS, but libsqlite3-sys and some build
# scripts still want pkg-config. No OpenSSL is linked into the binary.
RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Optional build identity shown in `app_version()`. Pass it from CI/release
# builds; the default keeps local builds working.
ARG BUILD_SHA=dev
ENV BUILD_SHA=${BUILD_SHA}

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

# curl is used by the container health check and by operators. sqlite3, tar,
# and gzip support the two shipped backup/restore scripts. No compiler,
# Node.js, pnpm, development scripts, docs or example environment file ship in
# the runtime image.
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates curl sqlite3 tar gzip findutils \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --create-home --home-dir /app braindrop \
    && mkdir -p /data /app/scripts \
    && chown braindrop:braindrop /data

WORKDIR /app
COPY --from=builder /usr/local/bin/braindrop-cli /usr/local/bin/braindrop-cli
COPY --from=builder /app/config/production.yaml ./config/production.yaml
COPY --from=frontend /app/frontend/dist ./frontend/dist
COPY scripts/backup.sh scripts/restore.sh ./scripts/

ENV LOCO_ENV=production \
    PORT=5150 \
    BINDING=0.0.0.0

USER braindrop
EXPOSE 5150
STOPSIGNAL SIGTERM

HEALTHCHECK --interval=15s --timeout=3s --start-period=15s --retries=4 \
    CMD curl -fsS "http://127.0.0.1:${PORT:-5150}/_health" || exit 1

ENTRYPOINT ["braindrop-cli"]
# One process serves HTTP, consumes the SQLite queue and runs the scheduler.
# Override with `--worker`/`--scheduler` for a split deployment.
CMD ["start", "--all"]
