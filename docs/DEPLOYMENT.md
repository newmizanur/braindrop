# BrainDrop Deployment

BrainDrop ships as one process with separate application and queue SQLite
databases. This document covers
four supported ways to run it:

| Option | Best for | Runtime dependencies on the host |
|---|---|---|
| Docker Compose | Always-on self-hosting | Docker + Compose |
| Native Alpine / OpenRC | Lightweight Alpine VPS | musl binary, `sqlite`, `curl`; OpenRC |
| Native Ubuntu / systemd | Conventional Ubuntu VPS | glibc binary, `sqlite3`, `curl`; systemd |
| Fly.io | Managed Docker hosting or low-cost demo | `flyctl` on the workstation |

All paths use the same app behavior, `config/production.yaml`, and
`frontend/dist`. Native archives can be installed by hand or updated over SSH
with `make deploy-*`; the workstation needs only `ssh` and `scp`. There is no
separate worker or scheduler container: Loco's
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
sudo -u braindrop env DATA_DIR=/opt/braindrop/data \
  BACKUP_DIR=/opt/braindrop/backup /opt/braindrop/scripts/backup.sh
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

Native packages are public distributions; the SSH deployer derives a separate
runtime-only archive. The runtime layout is flat and keeps service definitions
outside the application directory:

```text
/opt/braindrop/
  braindrop                 root:root, executable
  .env                      braindrop:braindrop, mode 600
  config/production.yaml    root-owned, readable
  frontend/dist/            root-owned, readable
  scripts/backup.sh         root-owned, executable
  scripts/restore.sh        root-owned, executable
  data/                     braindrop-writable SQLite files
  logs/                     braindrop-writable OpenRC logs
  backup/                   braindrop-writable backup archives
/etc/init.d/braindrop       Alpine OpenRC service
/etc/systemd/system/braindrop.service  Ubuntu systemd service
```

The service account owns only `.env` and the writable state directories. The
binary, configuration and static assets are root-owned and readable/executable
as needed. Archive extraction normalizes ownership and modes, so macOS archive
ownership and restrictive source umasks cannot make the binary unexecutable.
macOS metadata is removed. Docker uses a separate minimal Debian slim runtime
with the same compiled frontend and backup/restore tools; it does not embed the
frontend.

For a new Alpine host install `ca-certificates`, `curl`, `openssl`, `sqlite`,
`bash` and `tzdata`; OpenRC is required. For Ubuntu install `ca-certificates`,
`curl`, `openssl`, `sqlite3`, `bash` and `tzdata`; systemd is required. The
installer creates a dedicated unprivileged account, persistent `data/`,
`logs/`, and `backup/` directories, and service definitions outside
`/opt/braindrop`. Alpine uses `supervise-daemon`, respawns after failures and
writes logs under `logs/`; Ubuntu uses systemd with a 30-second graceful stop
and write access restricted to the state directories. Both run Loco
`start --all`.

## 4. SSH deployment

`make deploy-*` and `scripts/deploy.sh` install a flat runtime over plain `ssh`
and `scp`. They do not use `rsync`, Docker or a compiler on the server, and
they never disable SSH host-key verification. Public distribution archives
remain available separately and contain documentation and `.env.example`;
production receives only the binary, production config, compiled frontend,
service installer, migration helper and backup/restore scripts.

Remote layout:

```text
/opt/braindrop/
  braindrop, config/, frontend/, scripts/
  .env                            created once, preserved across upgrades
  data/                           application + queue SQLite and sidecars
  backup/                         DB backups and one code-only rollback archive
  logs/                           Alpine service logs
```

First install:

```sh
make deploy-init \
  SSH_HOST=server.example.com SSH_USER=root \
  OS=alpine ARCH=amd64
```

This builds and packages the matching target, uploads it, creates
`.env` if it does not exist, creates the writable state directories, and
generates `JWT_SECRET` and `AI_ENCRYPTION_KEY` with `openssl rand -base64 32`.
Registration starts disabled. `NOTIFICATION_ENCRYPTION_KEY` may remain empty to
use the supported AI encryption-key fallback. Review `.env` (especially `HOST`,
DB paths and optional mail settings) before exposing a new instance. Existing
environment files, databases, logs and backups are preserved and moved from
the former `shared/` layout when needed.

Routine update:

```sh
make deploy-linux \
  SSH_HOST=server.example.com SSH_USER=deploy \
  OS=ubuntu ARCH=arm64
```

The deploy action uploads a runtime-only staging archive, normalizes file
ownership and permissions, stops the service, takes an online SQLite backup,
runs migrations as the service user, then installs the flat runtime and
restarts the service. It keeps one code-only previous-runtime archive for
rollback. If health fails, the prior runtime is restored. **The database schema
is not rolled back**; restoring a schema-incompatible version may require the
matching pre-upgrade database backup. File replacement occurs while the service
is stopped, so a routine upgrade includes a short maintenance interval.

Upload staging is hardened: `scripts/deploy.sh` creates an unpredictable archive
path under the SSH user's home with `mktemp` and `umask 077` instead of a
predictable file under world-writable `/tmp`, extracts into a `0700` staging
directory, and removes both the uploaded archive and the staging directory on
success or failure. Host-key verification stays enabled.

Other targets:

```sh
make deploy-files    # upload without activating
make deploy-restart  # restart and health-check
make deploy-status   # flat layout, service state, health
make deploy-logs     # journalctl / tail -f logs/braindrop.log
make deploy-stop
make deploy-backup   # run scripts/backup.sh as the service user
make deploy-rollback # restore the previous runtime (no DB downgrade)
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
| `SKIP_BUILD` | `0` | reuse an existing `dist/` archive |
| `AUTO_ROLLBACK` | `1` | roll back on a failed health check |
| `GENERATE_SECRETS` | `1` | generate secrets during `init` |

Validate arguments without touching a server:

```sh
SSH_HOST=server.example.com SSH_USER=deploy OS=ubuntu ARCH=arm64 \
  ./scripts/deploy.sh validate
```

The script rejects empty or unsafe values and unsupported OS/architecture
combinations and out-of-range ports. `deploy-rollback` preserves the currently
active runtime as the next rollback target. Before deleting the old release
tree during migration, the script refuses to proceed if it finds `.env`,
SQLite files or state directories there.

### TierHive compatibility

TierHive-style hosts are supported through the same path: any host where you
have SSH access, `sh`, `tar`, `curl`, `sqlite3`/`sqlite` and `openssl` works.
Use `SSH_USER=root` (or a user with `sudo`) for `deploy-init`, then a
non-root deploy user for updates if the platform supports it. The deployment
does not assume a container runtime, `systemd` or OpenRC until the init script
is run, and it reports which service manager it found. No platform-specific
hostname, key or capability is guessed: verify them on the target first.

## 5. Fly.io Docker deployment

The root [`fly.toml`](../fly.toml) builds the existing production `Dockerfile`,
which runs Loco `start --all` as uid 10001. It configures one shared vCPU, 256
MiB, Singapore (`sin`), HTTPS, a `/_health` HTTP check, SIGTERM with a 30-second
shutdown window, and a 1 GiB volume named `braindrop_data` mounted at `/data`.
Both application and queue SQLite URLs point to that volume. The Dockerfile
creates `/data` and assigns it to the non-root user; Fly's actual mounted-volume
ownership still needs a real-machine verification. Do not use a release-command
machine: it has no attached volume, while app startup applies migrations to the
mounted databases.

Install [flyctl](https://fly.io/docs/flyctl/install/) and authenticate with
`fly auth login`. Review billing and region availability, then:

```sh
make fly-init
make fly-deploy
make fly-status
make fly-logs
```

`fly-init` creates the app and volume only if absent; resource creation can
incur charges. `fly-deploy` refuses to proceed if more than one machine exists
and deploys with `--ha=false`. It never destroys resources or configures
secrets. For custom app names, regions, volumes or memory, align `fly.toml` with
`FLY_APP`, `FLY_REGION`, `FLY_VOLUME` and `FLY_MEMORY`. The single-machine
constraint is enforced before deployment; Fly autostop does not create or
destroy machines. The persistent volume survives ordinary stops and image
deploys but is local to one host, is not replicated and may be unavailable
during host failures or deploys. Keep verified off-host backups.

For an application-level backup while online, run
`fly ssh console -a braindrop -C 'scripts/backup.sh'`; like every online
two-file run, the app and queue SQLite snapshots are sequential, not one atomic
snapshot. For a quiesced point-in-time copy of the entire volume, identify the
sole machine and attached volume, stop that machine, create a Fly volume
snapshot, then start the same machine:

```sh
fly machine list -a braindrop
fly volumes list -a braindrop
fly machine stop <machine-id> -a braindrop --signal SIGTERM --timeout 30
fly volumes snapshots create <volume-id> -a braindrop
fly machine start <machine-id> -a braindrop --wait
```

Restore a snapshot into a new volume, verify it during planned maintenance,
then change the configured mount only after confirming the snapshot and keeping
the original volume intact:

```sh
fly volumes create braindrop_data_restore -a braindrop -r sin -s 1 \
  --snapshot-id <snapshot-id>
# Change [mounts].source in fly.toml to braindrop_data_restore, then:
make fly-deploy
```

Keep the original volume until application and queue data are verified. Fly
snapshots are platform recovery points, not an off-host application backup;
keep a separate tested archive using `scripts/backup.sh`. Do not use `fly deploy`
release commands for volume-backed backup or migration work.

Demo behavior uses `auto_stop_machines = "stop"`,
`auto_start_machines = true`, and `min_machines_running = 0`. Fly Proxy checks
idle capacity every few minutes; the idle window is not a configurable
15-minute guarantee. A stopped machine cold-starts on a request. The scheduler
and queue worker cannot run while stopped. On restart, durable pending
deliveries are recovered, but reminders have a 24-hour grace window and daily
digests a 120-minute local-time delivery window. Longer stoppages can miss
notices. No application idle timer is used.

For installations requiring reliable reminders, digests and background work,
set:

```toml
auto_stop_machines = "off"
auto_start_machines = true
min_machines_running = 1
```

One attached SQLite volume still means one replica and some downtime during
deployments or host failures.

### Fly secrets and administrator setup

Configure sensitive values as Fly secrets. Never upload `.env` or commit real
values:

```sh
fly secrets set JWT_SECRET="$(openssl rand -base64 32)"
fly secrets set AI_ENCRYPTION_KEY="$(openssl rand -base64 32)"
# Optional: if unset, notification credentials use AI_ENCRYPTION_KEY.
fly secrets set NOTIFICATION_ENCRYPTION_KEY="$(openssl rand -base64 32)"
# Optional server-wide key; do not set this for a public BYOK demo.
fly secrets set OPENROUTER_API_KEY="..."
```

`JWT_SECRET` is required and must base64-decode to at least 32 random bytes.
`AI_ENCRYPTION_KEY` is optional for manual use, but BYOK requires 32 random
bytes encoded as base64. `NOTIFICATION_ENCRYPTION_KEY` is optional and falls
back to the AI key. Keep encryption keys in an access-limited encrypted
off-host secret backup and stable across deployments: losing or rotating one
makes existing encrypted user credentials unreadable. Rotating `JWT_SECRET`
invalidates sessions. Avoid putting generated values in shell history or logs.

SMTP is only needed for verification, password-reset or magic-link email. Set
`MAILER_ENABLED=true`, `MAILER_HOST`, `MAILER_PORT`, `MAILER_SECURE`, and the
optional `MAILER_USER`/`MAILER_PASSWORD`. To let users use personal OpenRouter
keys without a shared key, set non-secret `AI_ENABLED=true`, choose
`OPENROUTER_MODEL`, and store `AI_ENCRYPTION_KEY` as a secret. Manual capture
works with AI disabled.

Registration remains disabled. After deployment, create the first account
using an authenticated console. The task accepts `password_stdin:true`, which
reads one line from stdin instead of putting the password in process arguments.
Enter it without echo in the local shell:

```sh
read -r -s -p "Admin password: " ADMIN_PASSWORD
printf '\n'
printf '%s\n' "$ADMIN_PASSWORD" | fly ssh console -a braindrop \
  -C 'braindrop-cli task admin:create email:admin@example.com name:Admin password_stdin:true'
unset ADMIN_PASSWORD
```

The same stdin mode works through SSH for native installs. Never place an
administrator password in shell history, process arguments or captured logs,
and never publish shared administrator credentials.

### Public demo policy

Existing protections include disabled-by-default registration, authentication
rate limits, per-user AI request limits, a 1 MiB request-body cap, encrypted
write-only provider credentials and constrained provider destinations. Limits
are process-local, item creation has no account quota, SQLite growth has no
configured cap, and no demo account/data cleanup exists. Public access can
therefore invite abuse and sensitive data retention. If opened, keep
registration disabled until a manual account/retention process exists, do not
configure a shared OpenRouter key, require users to bring their own key, ask
users to use synthetic data, disclose cold starts and missed scheduled notices,
and make no production-availability promise. No public URL is published by this
task.

### Fly pricing estimate

Verified 2026-11-19 against [official Fly.io pricing](https://fly.io/docs/about/pricing/).
The published shared-cpu-1x/256 MiB reference is USD $2.19 per 30 days in the
lowest-priced regions; the documented Singapore 1 GiB example is priced
differently, and the 256 MiB Singapore quote should be checked in the current
calculator. A 1 GiB volume is $0.15/month, Asia-Pacific egress is $0.04/GB,
and volume snapshots cost $0.08/GB-month after the first 10 GB free. Stopped
machines still incur root-filesystem and volume charges. `$2.34` is only a
lowest-region always-on compute-plus-volume reference, not a Singapore quote or
a fixed bill; demo autostop reduces compute charges while stopped.

## 6. Configuration

Start from [`.env.example`](../.env.example). It is placeholder-only.

Required in production:

| Variable | Notes |
|---|---|
| `DATABASE_URL` | `sqlite:///data/braindrop.sqlite?mode=rwc` in Docker, `sqlite:///opt/braindrop/data/braindrop.sqlite?mode=rwc` natively |
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
- the deployment scripts generate them only when `.env` is created and
  never overwrite an existing file.

TLS is terminated outside the application: by a reverse proxy, the platform's
load balancer, or Docker's host proxy. BrainDrop does not manage certificates
and does **not** send `Strict-Transport-Security`; configure HSTS at the
TLS-terminating proxy before public exposure. Set `HOST` to the public HTTPS
origin and restrict the application port to the proxy where possible.

## 7. Database, queue and migrations

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

## 8. Backups and restore

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

## 9. Upgrades and rollback

Docker:

1. `docker compose exec app scripts/backup.sh`
2. pull or rebuild the image
3. `docker compose up -d`
4. verify `/_health`, login, item list, one notification test

Native/SSH:

1. `make deploy-backup`
2. `make deploy-linux` (or `deploy-files` then `deploy-linux` with `SKIP_BUILD=1`)
3. the script backs up the databases, migrates, replaces the flat runtime,
   restarts and health-checks
4. on failure it restores the previous code archive and restarts

`make deploy-rollback` switches to the previous runtime archive. It does **not**
downgrade the database. If the new release migrated the schema in a
non-backward-compatible way, restore the matching backup.

The deployer retains only one code rollback archive. Database backups remain
separate in `backup/`; operators should copy needed backups off-host.

## 10. Health and observability

- `GET /_ping` — liveness
- `GET /_health` — health (used by Docker, systemd health checks and deploy.sh)
- `GET /_readiness` — readiness

Startup logs must contain `worker is online` and `scheduler is running`.
CI fails the build if either is missing, so a server-only start cannot silently
disable reminders and scheduled scans.

Logs:

- Docker: `docker compose logs -f app`
- Alpine: `logs/braindrop.log` and `logs/braindrop.err.log`
- Ubuntu: `journalctl -u braindrop -f`

## 11. Security checklist

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
- [ ] `.env` mode `600`, owned by the service user
- [ ] Volume/database directories writable only by the service user
- [ ] SSH host-key verification left on; use `known_hosts`, not `StrictHostKeyChecking=no`
- [ ] Backups encrypted at rest and restore-tested
- [ ] One replica only
- [ ] Logs contain no secrets or tokens

## 12. Validation status

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

Native package permissions, runtime archive filtering, legacy state migration,
and non-root execution have regression tests in the CI container. Actual
OpenRC/systemd boot behaviour remains platform-specific and should be checked
on each target.
