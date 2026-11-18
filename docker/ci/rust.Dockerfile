# syntax=docker/dockerfile:1.7

# The Loco application refuses to boot when `frontend/dist` is missing, even in
# tests, because static serving is part of the app. Build the SPA first so the
# Rust checks run against the same assets a deployment serves.
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

FROM rust:1-bookworm

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        libssl-dev \
        pkg-config \
        sqlite3 \
    && rm -rf /var/lib/apt/lists/* \
    && rustup component add rustfmt clippy

WORKDIR /app
COPY . .
COPY --from=frontend /app/frontend/dist ./frontend/dist
RUN chmod +x scripts/*.sh

CMD ["./scripts/ci-rust.sh"]
