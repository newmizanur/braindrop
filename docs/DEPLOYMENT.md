# BrainDrop Deployment

> **Status: partly implemented.** The production multi-stage image and the
> Docker-first local CI pipeline exist as of milestone 2. Deployment Compose,
> GHCR publishing, backup scripts and the TierHive template are implemented in
> milestone 6. This document describes the target deployment contract so earlier
> milestones do not make decisions that block it.

## 1. Target

- One BrainDrop application replica.
- One SQLite application database on a persistent volume.
- One SQLite queue database on the same persistent volume.
- Separate server, worker and scheduler processes (or `--all` in a single
  container for small installations).
- No Redis, PostgreSQL, Nginx or Node.js in the runtime image.
- TLS terminated by the platform or a reverse proxy in front of BrainDrop.

SQLite is a single-writer database. Do **not** run multiple application replicas
against the same database or queue file.

## 2. Build and runtime model

Multi-stage image (implemented in `Dockerfile`):

1. **Frontend stage** — Node.js + pnpm: install from the frozen lockfile and
   build into `frontend/dist`.
2. **Rust build stage** — Rust stable: build the release binary with the frontend
   assets available. Loco refuses to boot without `frontend/dist`.
3. **Runtime stage** — Debian slim with the binary, `frontend/dist`, `config/`
   and CA certificates. Non-root user. No compiler, Node.js, pnpm or source
   tree.

The image declares a health check against `/_health`. CI starts it with a
disposable SQLite database, creates an administrator with `admin:create`, and
runs Playwright against it.

The runtime command is the Loco binary. Run modes:

```sh
braindrop-cli start                 # HTTP server + static SPA
braindrop-cli start --worker        # queue worker
braindrop-cli start --scheduler     # scheduled scans
braindrop-cli start --all           # all three in one process
```

## 3. Required environment

```env
# Runtime
PORT=5150
BINDING=0.0.0.0
HOST=https://braindrop.example.com
RUST_LOG=info

# Database and queue (persistent volume)
DATABASE_URL=sqlite:///data/braindrop.sqlite?mode=rwc
QUEUE_URL=sqlite:///data/braindrop_queue.sqlite?mode=rwc

# Auth
# JWT_SECRET must be base64: Loco decodes it before signing.
# Generate one with `openssl rand -base64 32`.
JWT_SECRET=<base64 secret>
ALLOW_REGISTRATION=false
REQUIRE_EMAIL_VERIFICATION=false

# Mail (verification / password reset)
MAILER_HOST=smtp.example.com
MAILER_PORT=587
MAILER_USER=...
MAILER_PASSWORD=...

# Optional AI
OPENROUTER_API_KEY=
OPENROUTER_MODEL=
OPENROUTER_BASE_URL=https://openrouter.ai/api/v1

# Integration credential encryption (required if Telegram/Discord are used)
SECRETS_KEY=<32-byte base64 or hex key>
```

Production config deliberately has no defaults for `DATABASE_URL`,
`QUEUE_URL`, `JWT_SECRET`, `HOST` or mailer credentials. The app refuses to boot
if they are missing.

Never commit a real `.env`. Ship `.env.example` with placeholders only.

## 4. Persistent volume

Mount a single volume at `/data` (or an equivalent path) and keep both SQLite
files there, including WAL/SHM sidecars:

```text
/data/braindrop.sqlite
/data/braindrop.sqlite-wal
/data/braindrop.sqlite-shm
/data/braindrop_queue.sqlite
/data/braindrop_queue.sqlite-wal
/data/braindrop_queue.sqlite-shm
```

The container runs as a non-root user; the volume must be writable by that uid.

## 5. Health and monitoring

Loco provides:

- `GET /_ping` — liveness
- `GET /_health` — health
- `GET /_readiness` — readiness

Use `/_health` for container health checks and `/_readiness` for traffic
readiness. Do not expose these as the only auth boundary; they are monitoring
endpoints, not application data.

## 6. Backups

SQLite must be backed up with a consistent mechanism, not by copying a live file
while writes are in flight.

Target approach (milestone 6):

```sh
# Online backup into a temporary file, then compress and move off-host.
sqlite3 /data/braindrop.sqlite ".backup '/data/backups/braindrop-$(date -u +%Y%m%dT%H%M%SZ).sqlite'"
```

- Use SQLite's `.backup` (online backup API), not `cp`, for a live database.
- Back up the queue database only if in-flight jobs must be preserved; delivery
  state is recoverable from the application tables.
- Verify the backup with `PRAGMA integrity_check`.
- Encrypt backups at rest and store them outside the application volume.
- Document retention and restore drills.

Restore:

1. Stop server, worker and scheduler.
2. Move the current database aside (do not delete it).
3. Restore the selected backup to `DATABASE_URL`.
4. Run `braindrop-cli db migrate` if the backup predates the current release.
5. Start the app and verify `/_health`, login and item counts.

## 7. Migrations

- Development and small single-instance deployments: `auto_migrate: true`.
- Multiple processes/releases: run migrations once as a release step with
  `braindrop-cli db migrate`, then start the app with auto-migrate disabled.
- Never run two migrating processes against the same SQLite file concurrently.

## 8. CI and image publishing

Docker-first local CI is implemented (milestone 2):

- `./scripts/ci.sh` builds `docker-compose.ci.yml` and the production image.
- It runs Rust fmt/clippy/tests and a throwaway SQLite migration, frontend
  typecheck/lint/Vitest/build, a production-image health check with a seeded
  login, and Playwright smoke tests.
- Cargo caches live in named Docker volumes and the pnpm store in a BuildKit
  cache mount; no development or
  production database is mounted.

`.github/workflows/ci.yaml` is manual-only (`workflow_dispatch`) and calls
`./scripts/ci.sh`. It has no Redis, PostgreSQL or npm steps. Automatic runs stay
disabled until explicitly re-enabled.

Milestone 6 adds publishing:

- Tag/main builds publish `ghcr.io/<owner>/braindrop:<tag>` using
  `GITHUB_TOKEN`.

CI must not require real OpenRouter, Telegram, Discord or SMTP credentials.

## 9. TierHive / SSH deployment template

The template will be added in milestone 6 and must be verified against the
actual TierHive platform before activation. It should:

- build or pull an immutable image tag;
- copy `.env` from a secret store, never from the repository;
- create/attach the persistent volume;
- run migrations as a one-shot step;
- start server, worker and scheduler;
- health-check `/_health`;
- keep the previous image for rollback.

Do not guess hostnames, SSH users, keys or platform capabilities. Do not execute a
production deployment without explicit authorization.

## 10. Upgrade procedure

1. Back up the database.
2. Pull the new image.
3. Run migrations (one process).
4. Restart server, worker and scheduler.
5. Verify health, login, item list and a test notification.
6. Keep the previous image available for rollback.

## 11. Security checklist

- [ ] TLS enabled in front of the app
- [ ] Strong `JWT_SECRET` and `SECRETS_KEY`
- [ ] Public registration disabled unless intended
- [ ] Email verification policy chosen
- [ ] SMTP credentials supplied via secrets
- [ ] AI key optional; feature disabled when absent
- [ ] Telegram/Discord credentials encrypted at rest
- [ ] Volume permissions restricted to the runtime user
- [ ] Backups encrypted and tested
- [ ] Logs contain no secrets or tokens
- [ ] Single replica only
