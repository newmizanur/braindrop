# BrainDrop Deployment

BrainDrop ships as one process and one SQLite database. This document covers the
three supported ways to run it in production:

| Option | Best for | Runtime dependencies on the host |
|---|---|---|
| Docker Compose | Most self-hosted installs | Docker + Compose |
| Native binary (Alpine / Ubuntu) | Small VPS, no container runtime | libc (glibc or musl), `sqlite3`, `curl` |
| SSH deployment (`make deploy-*`) | Repeatable updates over SSH | `ssh`/`scp` on the workstation; no Docker, Rust, Node.js or `rsync` on the server |

All three run the same release binary, the same `config/production.yaml` and the
same `frontend/dist`. There is no separate worker or scheduler container: Loco's
`start --all` runs the HTTP server, the SQLite queue worker and the scheduler in
one process.

> **Single replica only.** SQLite is a single-writer database. Never point two
> application processes at the same database or queue file. If you need more
> throughput than one node provides, that is a different architecture, not a
> second replica.

## 1. What runs

```sh
braindrop start --all        # HTTP + static SPA + queue worker + scheduler
braindrop start --worker     # worker only (split deployments)
braindrop start --scheduler  # scheduler only (split deployments)
braindrop db migrate         # apply migrations once, as a release step
```

Runtime files:

```text
braindrop                     release binary (renamed from braindrop-cli)
config/production.yaml        production configuration
frontend/dist/                built SPA (served by Loco's static middleware)
scripts/backup.sh             consistent SQLite backup
scripts/restore.sh            restore with confirmation
.env.example                  placeholder configuration
```

Mail templates are compiled into the binary with `include_dir!`. No source tree,
Node.js, pnpm, Rust toolchain, Redis or PostgreSQL is needed at runtime.

The SPA is served from `frontend/dist` on disk, not embedded in the binary. This
is deliberate: Loco 1.3.1's `embedded_assets` build only discovers an `assets/`
directory, and moving the SPA there would fight the generated frontend layout.
Keeping `frontend/dist` alongside the binary is the supported path and is what
every distribution archive contains.

## 2. Docker Compose

The repository root has a production `docker-compose.yml` and a separate
`docker-compose.ci.yml`. Only the former is for real deployments.

```sh
cp .env.example .env
# edit .env: set JWT_SECRET, HOST, and any optional integrations
docker compose up -d
docker compose ps
curl -fsS http://localhost:5150/_health
```

What the Compose file gives you:

- one `app` service built from the multi-stage `Dockerfile`;
- `env_file: .env` — the container never reads a `.env` baked into the image;
- a named volume `braindrop-data` mounted at `/data` for the application
  database, the queue database and their WAL/SHM sidecars;
- the host port and the container port both taken from `PORT`, so `HOST`, the
  health check and a reverse proxy cannot disagree;
- `restart: unless-stopped`, `init: true` and a 30-second stop grace period so
  Loco's SIGTERM handler can drain the worker and scheduler;
- a health check against `/_health`.

The image is multi-stage:

1. `node:22-bookworm-slim` + pnpm builds `frontend/dist` from the frozen
   lockfile.
2. `rust:1-bookworm` builds `braindrop-cli` in release mode.
3. `debian:bookworm-slim` runs as the unprivileged user `braindrop` (uid 10001)
   with `ca-certificates`, `curl`, `sqlite3`, `tar`, `gzip` and `findutils`.
   There is no compiler, Node.js or pnpm in the final image.

Build for another architecture with BuildKit:

```sh
docker buildx build --platform linux/amd64 -t braindrop:amd64 .
docker buildx build --platform linux/arm64 -t braindrop:arm64 .
```

Upgrade:

```sh
docker compose pull          # when using a published image
docker compose up -d         # rebuilds if you build locally
docker compose logs -f app
```

`docker compose down` keeps the named volume. `docker compose down -v` deletes
your data.

### Running backups in the container

```sh
docker compose exec app scripts/backup.sh
docker compose exec app ls -l /data/backups
```

The image contains `sqlite3`, so the backup script uses the online backup API
rather than copying a live file.

The script backs up the queue database first and the application database
second. Each `.backup` is individually consistent, but SQLite cannot make
these two separate files one atomic snapshot. BrainDrop persists a notification
delivery before enqueueing its queue job, and the recovery scan requeues pending
deliveries after a restore; queue-first ordering therefore avoids restoring a
job without its app delivery row. A scheduler/worker can still change state
between the two snapshots.

For stronger consistency, stop the BrainDrop process before running the script
and start it again after the backup completes:

```sh
# Docker Compose
docker compose stop app
docker compose run --rm --no-deps app scripts/backup.sh
docker compose start app

# Native systemd / OpenRC
sudo systemctl stop braindrop   # Ubuntu; use `sudo rc-service braindrop stop` on Alpine
sudo -u braindrop /opt/braindrop/current/scripts/backup.sh
sudo systemctl start braindrop # Ubuntu; use `sudo rc-service braindrop start` on Alpine
```

Stopping the process prevents both the scheduler and queue worker from changing
either database during the pair of online backups. Keep the backup on the same
host only as a staging copy; copy archives off-host and protect them as sensitive
user data.

## 3. Native binaries

Six release targets are supported:

| Target | Platform name in `dist/` | Typical host |
|---|---|---|
| `x86_64-unknown-linux-musl` | `linux-amd64-musl` | Alpine AMD64 |
| `aarch64-unknown-linux-musl` | `linux-arm64-musl` | Alpine ARM64 |
| `x86_64-unknown-linux-gnu` | `linux-amd64-glibc` | Ubuntu AMD64 |
| `aarch64-unknown-linux-gnu` | `linux-arm64-glibc` | Ubuntu ARM64 |
| `aarch64-apple-darwin` | `macos-arm64` | Apple Silicon macOS |
| `x86_64-apple-darwin` | `macos-amd64` | Intel macOS |

Build with Docker (no Linux toolchain on the workstation):

```sh
make build-alpine-amd64     # x86_64-unknown-linux-musl
make build-alpine-arm64     # aarch64-unknown-linux-musl
make build-linux-amd64      # x86_64-unknown-linux-gnu
make build-linux-arm64      # aarch64-unknown-linux-gnu
make build-mac              # both macOS targets, built natively
make build-all              # all six
```

The glibc builds use a `rust:1-bullseye` image on purpose: the glibc baseline is
2.31, which covers Ubuntu 20.04 and newer. Building on a newer distribution would
silently raise the minimum glibc version. Building an amd64 image on an Apple
Silicon host runs the compiler under QEMU and is slow but correct.

The toolchain image installs nothing. `rust:1-bullseye` already ships `cc`,
`pkg-config` and a CA bundle, and Debian 11 has moved to archive.debian.org, so an
`apt-get update` would fail. No system library is required: SQLite is bundled,
TLS is rustls and there is no OpenSSL dependency.

Package one target into a self-contained directory and archive:

```sh
./scripts/build-native.sh x86_64-unknown-linux-musl
./scripts/package.sh x86_64-unknown-linux-musl
```

Result:

```text
dist/braindrop-linux-amd64-musl/
  braindrop
  config/production.yaml
  frontend/dist/
  scripts/backup.sh
  scripts/restore.sh
  scripts/test-backup-restore.sh
  scripts/deploy-init-alpine.sh
  scripts/deploy-init-ubuntu.sh
  .env.example
  README.md
  DEPLOYMENT.md
  LICENSE
  RELEASE
dist/braindrop-linux-amd64-musl.tar.gz
dist/braindrop-linux-amd64-musl.tar.gz.sha256
```

The archive never contains a database, a real `.env`, `node_modules`, source or
build output. `scripts/package.sh --check <dir>` verifies the layout.

### Installing by hand on Alpine (OpenRC)

```sh
apk add ca-certificates curl openssl sqlite tzdata
tar -xzf braindrop-linux-amd64-musl.tar.gz -C /opt
ln -s /opt/braindrop-linux-amd64-musl /opt/braindrop
mkdir -p /opt/braindrop/shared/{data,backups,logs}
cp /opt/braindrop/.env.example /opt/braindrop/shared/.env
# edit shared/.env: DATABASE_URL/QUEUE_URL point at shared/data
# and generate JWT_SECRET, AI_ENCRYPTION_KEY, NOTIFICATION_ENCRYPTION_KEY
/opt/braindrop/scripts/deploy-init-alpine.sh
```

### Installing by hand on Ubuntu (systemd)

```sh
apt-get install -y ca-certificates curl openssl sqlite3 tzdata
tar -xzf braindrop-linux-amd64-glibc.tar.gz -C /opt
ln -s /opt/braindrop-linux-amd64-glibc /opt/braindrop
mkdir -p /opt/braindrop/shared/{data,backups,logs}
cp /opt/braindrop/.env.example /opt/braindrop/shared/.env
# edit shared/.env
/opt/braindrop/scripts/deploy-init-ubuntu.sh
```

Both installers are idempotent. They create a dedicated unprivileged
`braindrop` user, set ownership and permissions on `shared/`, and install a
service definition:

- **Alpine**: `/etc/init.d/braindrop` using `supervise-daemon`, `respawn_delay=5`,
  `need net`, logs in `shared/logs/`.
- **Ubuntu**: `/etc/systemd/system/braindrop.service` with `Type=simple`,
  `Restart=on-failure`, `RestartSec=5`, `TimeoutStopSec=30` and hardening
  (`NoNewPrivileges`, `PrivateTmp`, `ProtectSystem=strict`,
  `ReadWritePaths=/opt/braindrop/shared`, ...). Logs go to journald.

Neither installer disables host security features and neither writes secrets
into the unit file: both read `shared/.env`.

## 4. SSH deployment

`make deploy-*` and `scripts/deploy.sh` implement a release-based deployment
over plain `ssh` and `scp`. They do not use `rsync`, Docker or a compiler on the
server, and they never disable SSH host-key verification.

Remote layout:

```text
/opt/braindrop/
  current -> releases/<release-id>/
  releases/<release-id>/          immutable unpacked release
  shared/.env                     created once, preserved forever
  shared/data/                    application + queue SQLite and sidecars
  shared/backups/                 archives from scripts/backup.sh
  shared/logs/                    Alpine service logs
```

First install:

```sh
make deploy-init \
  SSH_HOST=server.example.com SSH_USER=root \
  OS=alpine ARCH=amd64
```

This builds and packages the matching target, uploads it, creates
`shared/.env` from `.env.example` with `DATABASE_URL`/`QUEUE_URL` rewritten to
`shared/data`, generates `JWT_SECRET`, `AI_ENCRYPTION_KEY` and
`NOTIFICATION_ENCRYPTION_KEY` with `openssl rand -base64 32`, activates the
release, installs the service and waits for `/_health`. It then tells you to
review `shared/.env` (in particular `HOST`) before exposing the service.

Routine update:

```sh
make deploy-linux \
  SSH_HOST=server.example.com SSH_USER=deploy \
  OS=ubuntu ARCH=arm64
```

The deploy action uploads to a staging directory, extracts there, runs
migrations as the service user, atomically repoints `current`, restarts the
service and polls `/_health`. If the health check fails it repoints `current`
back to the previous release and restarts. **The database is not rolled back**;
a failed migration needs a backup restore, not a symlink flip.

Upload staging is hardened: `scripts/deploy.sh` creates an unpredictable archive
path under the SSH user's home with `mktemp` and `umask 077` instead of a
predictable file under world-writable `/tmp`, extracts into a `0700` staging
directory, and removes both the uploaded archive and the staging directory on
success or failure. Host-key verification stays enabled.

Other targets:

```sh
make deploy-files    # upload without activating
make deploy-restart  # restart and health-check
make deploy-status   # current release, service state, health
make deploy-logs     # journalctl / tail -f shared/logs/braindrop.log
make deploy-stop
make deploy-backup   # run scripts/backup.sh as the service user
make deploy-rollback # activate the previous release (no DB downgrade)
```

Variables (all overridable on the command line or via the environment):

| Variable | Default | Meaning |
|---|---|---|
| `SSH_HOST` | — | hostname or IP, required |
| `SSH_PORT` | `22` | SSH port |
| `SSH_USER` | `root` | SSH user; `REMOTE_SUDO` defaults to `sudo` for non-root |
| `SSH_IDENTITY` | — | optional private key |
| `REMOTE_DIR` | `/opt/braindrop` | remote root |
| `OS` | `alpine` | `alpine` or `ubuntu` |
| `ARCH` | `amd64` | `amd64` or `arm64` |
| `SERVICE` | `braindrop` | service name |
| `SERVICE_USER` | `braindrop` | unprivileged runtime user |
| `APP_PORT` | `5150` | port used for the remote health check |
| `KEEP_RELEASES` | `3` | old releases to keep |
| `SKIP_BUILD` | `0` | reuse an existing `dist/` archive |
| `AUTO_ROLLBACK` | `1` | roll back on a failed health check |
| `GENERATE_SECRETS` | `1` | generate secrets during `init` |

Validate arguments without touching a server:

```sh
SSH_HOST=server.example.com SSH_USER=deploy OS=ubuntu ARCH=arm64 \
  ./scripts/deploy.sh validate
```

The script rejects empty or unsafe values, unsupported OS/architecture
combinations, out-of-range ports and an empty `KEEP_RELEASES`.

### TierHive compatibility

TierHive-style hosts are supported through the same path: any host where you
have SSH access, `sh`, `tar`, `curl`, `sqlite3`/`sqlite` and `openssl` works.
Use `SSH_USER=root` (or a user with `sudo`) for `deploy-init`, then a
non-root deploy user for updates if the platform supports it. The deployment
does not assume a container runtime, `systemd` or OpenRC until the init script
is run, and it reports which service manager it found. No platform-specific
hostname, key or capability is guessed: verify them on the target first.

## 5. Configuration

Start from [`.env.example`](../.env.example). It is placeholder-only.

Required in production:

| Variable | Notes |
|---|---|
| `DATABASE_URL` | `sqlite:///data/braindrop.sqlite?mode=rwc` in Docker, `sqlite:///opt/braindrop/shared/data/braindrop.sqlite?mode=rwc` natively |
| `QUEUE_URL` | same shape, separate file |
| `JWT_SECRET` | base64; Loco decodes it before signing. `openssl rand -base64 32` |
| `HOST` | public HTTPS origin used in outgoing mail links |

Optional:

- `PORT`, `BINDING`, `LOG_LEVEL`, `DB_AUTO_MIGRATE`, `DB_MAX_CONNECTIONS`
- `ALLOW_REGISTRATION`, `REQUIRE_EMAIL_VERIFICATION`
- `REMOTE_IP_SOURCE` — client-address source for authentication rate limits;
  `ConnectInfo` (the socket peer) by default, as in `.env.example`. See
  "Trusted proxies" in section 10 before changing it.
- `MAILER_ENABLED`, `MAILER_HOST`, `MAILER_PORT`, `MAILER_SECURE`,
  `MAILER_USER`, `MAILER_PASSWORD`
- `AI_ENABLED`, `OPENROUTER_API_KEY`, `OPENROUTER_MODEL`,
  `OPENROUTER_BASE_URL`, `AI_TIMEOUT_SECONDS`, `AI_MAX_INPUT_CHARS`,
  `AI_RATE_LIMIT_PER_MINUTE`, `AI_ENCRYPTION_KEY`
- `NOTIFICATION_ENCRYPTION_KEY` and the notification retry/retention settings

Mail is **disabled by default**. A manual-only install with registration off
boots without SMTP. Set `MAILER_ENABLED=true` before enabling email verification,
password reset or magic links.

`AI_ENCRYPTION_KEY` and `NOTIFICATION_ENCRYPTION_KEY` are long-lived secrets:

- they must be set once and backed up;
- rotating them makes stored personal OpenRouter keys and Telegram/Discord
  credentials unreadable;
- the deployment scripts generate them only when `shared/.env` is created and
  never overwrite an existing file.

TLS is terminated outside the application: by a reverse proxy, the platform's
load balancer, or Docker's host proxy. BrainDrop does not manage certificates
and does **not** send `Strict-Transport-Security`; configure HSTS at the
TLS-terminating proxy before public exposure. Set `HOST` to the public HTTPS
origin and restrict the application port to the proxy where possible.

## 6. Database, queue and migrations

Both databases live on the same persistent volume:

```text
braindrop.sqlite        application data
braindrop_queue.sqlite   Loco queue
*-wal, *-shm             SQLite sidecars
```

- `DB_AUTO_MIGRATE=true` is convenient for a single-node install: the app
  migrates on boot.
- For the SSH release flow, migrations also run explicitly as a release step
  before the new release is activated. If you prefer strict control, set
  `DB_AUTO_MIGRATE=false` and keep using `braindrop db migrate`.
- Never run two migrating processes against the same SQLite file.

The queue holds notification deliveries. Delivery rows are written to the
application database before a job is enqueued, and a recovery scan requeues
pending or abandoned deliveries after a restart, so losing the queue file is
recoverable. Back it up anyway if you want in-flight jobs preserved exactly.

## 7. Backups and restore

```sh
# Docker
docker compose exec app scripts/backup.sh

# Native / SSH
make deploy-backup SSH_HOST=... SSH_USER=...
```

`scripts/backup.sh`:

- uses SQLite's online backup API (`.backup`) for both databases — never `cp`
  on a live file;
- runs `PRAGMA integrity_check` on every copy;
- writes a timestamped `braindrop-backup-<UTC>.tar.gz` with a manifest and a
  `.sha256` sidecar;
- applies `RETENTION_DAYS` (default 30);
- optionally includes `.env` with `INCLUDE_ENV=1` (contains secrets; encrypt the
  archive at rest).

Restore:

```sh
./scripts/restore.sh /path/to/braindrop-backup-<UTC>.tar.gz
# type RESTORE to confirm
```

`scripts/restore.sh`:

- verifies the archive checksum and database integrity;
- refuses to run without an explicit confirmation (or `--yes` for automation);
- stops the service, moves the current databases aside into
  `restore-previous-<UTC>/` instead of deleting them;
- restores both databases, fixes ownership, optionally runs migrations
  (`RUN_MIGRATIONS=1 APP_BIN=...`) and restarts the service;
- optionally polls `HEALTH_URL`.

The repository self-tests this flow with real rows:

```sh
./scripts/test-backup-restore.sh
```

Restore drills matter: a backup that has never been restored is a hypothesis.

## 8. Upgrades and rollback

Docker:

1. `docker compose exec app scripts/backup.sh`
2. pull or rebuild the image
3. `docker compose up -d`
4. verify `/_health`, login, item list, one notification test

Native/SSH:

1. `make deploy-backup`
2. `make deploy-linux` (or `deploy-files` then `deploy-linux` with `SKIP_BUILD=1`)
3. the script migrates, activates, restarts and health-checks
4. on failure it automatically repoints `current` to the previous release

`make deploy-rollback` switches to the previous release. It does **not**
downgrade the database. If the new release migrated the schema in a
non-backward-compatible way, restore the matching backup.

Old releases beyond `KEEP_RELEASES` are pruned; the active release is never
pruned.

## 9. Health and observability

- `GET /_ping` — liveness
- `GET /_health` — health (used by Docker, systemd health checks and deploy.sh)
- `GET /_readiness` — readiness

Startup logs must contain `worker is online` and `scheduler is running`.
CI fails the build if either is missing, so a server-only start cannot silently
disable reminders and scheduled scans.

Logs:

- Docker: `docker compose logs -f app`
- Alpine: `shared/logs/braindrop.log` and `shared/logs/braindrop.err.log`
- Ubuntu: `journalctl -u braindrop -f`

## 10. Security checklist

### Application-provided hardening

Every environment enables Loco's `remote_ip` and `secure_headers` middleware plus
the application's cache-control layer:

- a strict `Content-Security-Policy` (`default-src 'self'`, `script-src 'self'`,
  `script-src-attr 'none'`, no inline scripts or `unsafe-eval`, `object-src
  'none'`, `base-uri 'none'`, `frame-ancestors 'none'`, `form-action 'self'`);
  style requests still allow `'unsafe-inline'` for Mantine;
- `X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY`,
  `Referrer-Policy: no-referrer`, `Permissions-Policy`,
  `Cross-Origin-Opener-Policy: same-origin` and
  `Cross-Origin-Resource-Policy: same-origin`;
- `Cache-Control: no-store` on `/api/*`, monitoring paths (`/_*`) and served
  HTML, while hashed static assets remain cacheable. The service worker also
  bypasses API and monitoring paths;
- a 1 MiB request-body limit and validated auth inputs (email format, password
  and name lengths), with unknown-email logins taking a dummy password-hash
  path to reduce timing-based account enumeration;
- in-process rate limits on authentication endpoints, keyed by account/token and
  client address;
- disabled redirects in the OpenRouter client, because the operator-configured
  base URL may be used with a secret key;
- redacted database error strings in notification API errors and delivery
  records (details stay in server logs).

The JWT still lives in `localStorage`, as generated by Loco. The CSP and
no-store policy reduce but do not remove that residual XSS/token-theft risk;
treat any XSS as a potential session compromise.

### Trusted proxies

`REMOTE_IP_SOURCE` defaults to `ConnectInfo` (socket peer), which is correct
when the app is exposed directly or through a proxy that preserves the
connection address. Set it to `RightmostXForwardedFor` **only** behind exactly
one trusted reverse proxy that overwrites `X-Forwarded-For`. Never trust a
forwarding header that a client can set, and do not expose the application port
publicly when using header-based sources. Authentication rate-limit counters are
in-process and reset on restart, so a public deployment should also limit
requests at the edge. Only one application replica is supported.

### Pre-deployment checks

- [ ] TLS in front of the app, `HOST` set to the public HTTPS origin, HSTS set
      at the proxy, and the application port restricted to the proxy
- [ ] `REMOTE_IP_SOURCE` appropriate for the network path; edge rate limits in
      place if publicly exposed
- [ ] `JWT_SECRET` generated with `openssl rand -base64 32` and backed up
- [ ] `AI_ENCRYPTION_KEY` / `NOTIFICATION_ENCRYPTION_KEY` set once and backed up
- [ ] `ALLOW_REGISTRATION=false` unless public sign-up is intended
- [ ] Mailer enabled only when needed; SMTP credentials supplied via `.env`
- [ ] AI disabled or given a server key; per-user keys need an encryption key
- [ ] `shared/.env` mode `600`, owned by the service user
- [ ] Volume/database directories writable only by the service user
- [ ] SSH host-key verification left on; use `known_hosts`, not `StrictHostKeyChecking=no`
- [ ] Backups encrypted at rest and restore-tested
- [ ] One replica only
- [ ] Logs contain no secrets or tokens

## 11. Validation status

What the project actually verifies today:

- **Docker**: `./scripts/ci.sh` builds the production image, starts it with
  `start --all`, checks `/_health`, the SPA and a deep link, creates an item,
  restarts the container, confirms the item survived, and runs an in-container
  backup.
- **Rust request tests**: assert the security headers and the `no-store` cache
  policy on API and HTML responses.
- **Backup/restore**: `scripts/test-backup-restore.sh` runs in CI against real
  SQLite files with items, settings and a pending queue job.
- **Deploy scripts**: argument validation and shell syntax checks run in CI.
- **Native targets**: cross-compiled with the documented toolchains and packaged
  with checksums. The macOS binaries are executed on the build host; Linux
  binaries are built in Linux containers and inspected, but the full native
  service flow is not exercised by CI.

Not verified automatically: an actual SSH deployment to a remote host, an actual
Alpine/Ubuntu boot of the init scripts, and TierHive platform behaviour. Those
require an authorized target and are documented rather than claimed.
