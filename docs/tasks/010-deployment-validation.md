# Task 010 — Deployment validation record

**Date:** 2026-11-19  
**Baseline:** Task 009 commit `103c591969b52f93c8a5254f50edfc5ca23105e6`  
**Scope:** Local Task 010 preparation plus authorized TierHive inspection and
Fly.io deployment validation. No repository contains remote addresses, SSH
credentials, encryption keys or administrator passwords. A publicly reachable
Fly deployment exists, but no demo URL was added to README and no credentials
were published.

## Implementation summary

- Added root `fly.toml` for the existing production Dockerfile, Loco
  `start --all`, `sin`, one shared CPU, 256 MiB, HTTPS/health checks, SIGTERM,
  and the 1 GiB `braindrop_data` mount at `/data`. The app and queue SQLite
  files both live on the mount; demo autostop is enabled. There is no
  release-command machine.
- Added idempotent Fly Makefile targets for initialization, deployment, status,
  logs, SSH and restart. Initialization preserves an existing app/volume;
  deployment refuses more than one machine and uses `--ha=false`. It never
  deletes resources or configures secrets.
- Documented Fly secret formats, SMTP/OpenRouter configuration, key backup and
  rotation, administrator creation, demo versus always-on scheduling, public
  demo controls, backup limitations, and current Fly pricing references.
- Added [`docs/BENCHMARK.md`](../BENCHMARK.md), preserving the Task 009 pool
  results and clearly separating the new local constrained-container sample.
- Added concise README performance notes and links to benchmark and deployment
  records; updated architecture, progress and `.env.example` guidance.
- Enabled existing per-user AI/BYOK in Fly config (`AI_ENABLED=true`); no
  shared OpenRouter API key is configured.

## Fly.io configuration and validation

`fly.toml` uses the checked-in production `Dockerfile`, which starts
`braindrop-cli start --all` as uid 10001. The deployed config uses port 5150,
`/_health`, `FlyClientIp`, both SQLite files under `/data`, and the requested
auto-stop/autostart/zero-minimum demo settings. `AI_ENABLED=true` enables the
existing personal-key BYOK path; there is no shared OpenRouter key. Strict Fly
configuration validation passed directly and during deployment.

Authorized provisioning created the app in the existing personal organization
and one encrypted 1 GiB volume in `sin`. Fresh JWT and AI encryption secrets
were generated using OpenSSL and piped directly to `fly secrets import`; their
values were neither printed nor saved. Fly built and deployed the production
image (reported size 37 MB). Exactly one machine is present: shared-cpu-1x,
one CPU, 256 MiB, attached to the volume, with one passing health check. The
mounted `/data` was owned by uid 10001/gid 999, mode 755. The app process ran
as uid 10001.

External HTTPS `/_health` and `/` returned 200. A valid-shaped registration
probe returned 403 `registration_disabled`. An explicit machine restart passed
health. Both SQLite databases remained present and passed `PRAGMA
integrity_check` after restart. After an authorized explicit stop, a health
request triggered autostart and returned 200 in 2.25 seconds. This validates
cold-start, not actual idle-stop timing; Fly native idle detection is enabled,
and its interval is not configurable to exactly 15 minutes. Administrator authentication, item create/read persistence, active resource
use and notification delivery remain untested because the initial administrator
identity is awaiting operator selection. Backup and restore passed against
disposable copies of the two mounted SQLite databases (4.3 KiB archive); this
did not include pending notifications or live delivery-boundary recovery.

Pricing was checked against official Fly documentation on 2026-11-19. The
published 256 MiB shared-cpu-1x baseline is $2.19 per 30 days in the cheapest
regions; the Singapore 1 GiB example is $8.50 per 30 days. The readable docs
do not expose a Singapore 256 MiB quote, so approximately $2.80/month for
continuous compute is an estimate inferred from regional price differences,
not a quoted rate. The 1 GiB volume is $0.15/month, Asia-Pacific egress is
$0.04/GB, and snapshot storage is $0.08/GB-month after the first 10 GB free.
Autostop reduces compute while stopped, but rootfs and volume charges continue.
The continuous-run budget is about $2.95/month plus egress/snapshots; actual
costs vary. Sources: [Fly pricing](https://docs.fly.io/about/pricing/) and
[Fly volumes](https://fly.io/volumes/).

## Make targets

`make help` lists all six Fly targets. Init and deploy ran against the
authorized account. Initial init required an organization for noninteractive
app creation; after the app was created in the existing personal org, rerunning
the target preserved it and created the named volume. Deploy built the image,
launched one machine, and applied the 256 MiB setting. Status, SSH, and restart
were exercised; restart passed. No resources were destroyed.

## TierHive results

Authorized inspection found Alpine 3.24.4, x86_64, 93.2 MiB RAM, 128 MiB swap,
and a 953 MiB root filesystem. OpenRC was active; standard network, time and
SSH services were present. No unrelated app service or local port conflict
was found. `ca-certificates`, `openssl` and `sqlite` were installed. The
x86_64 musl release was staged using the existing Makefile/native scripts. The
remote production env file was created with mode 600, fresh persistent JWT/AI
keys and registration disabled. OpenRC was registered and enabled at boot.

Startup did not open port 5150: metadata checks showed no app process, listener
or DB files, and deployment health failed. The automatic approval review
rejected retrieval of service log contents because they could contain
credentials or personal data; those logs were not read. The managed HAProxy
panel/upstream port was inaccessible from the host, and DNS checks found no
record for the user-provided domain. External HTTPS and proxy headers could
not be verified. The release remains staged and OpenRC is configured, but
service operation is **NOT VALIDATED**. No Nginx/Caddy/Certbot or unrelated
services were installed or changed. Admin/API, backup/restore, upgrade/rollback,
worker/scheduler runtime and resource checks are **BLOCKED** on startup and
domain/proxy prerequisites.

## Benchmark and resource measurements

The Task 009 pool results remain unchanged in `docs/BENCHMARK.md`; remote
measurements are separate. TierHive host metadata and Fly idle process RSS/database
sizes are recorded, but neither host has a validated authenticated workload or
peak resource sample.

An additional local production Docker container was tested with a 128 MiB
cgroup memory limit, one CPU, temporary SQLite files, Loco server/worker/
scheduler (`start --all`), and a small authenticated workload (40 item creates
and 40 list reads). Health passed; all HTTP requests succeeded; Docker reported
12.69 MiB at the sampled time, `OOMKilled=false`; the process used uid 10001 and
the bind-mounted data directory was owned by `10001:999`. The app+queue DB files
totaled approximately 152 KiB at sample. Request p50/p95 was 2.10/3.96 ms for
this small local sample. A comparable 256 MiB local Docker sample also
completed: 12.70 MiB sampled container memory, no OOM kill, API p50/p95
2.15/4.59 ms, and the same 40 writes and 40 reads. Neither sample is a peak
memory, active queue-job throughput, real provider workload, sustained capacity,
or production reliability claim. There is no before/after benchmark.

The locally built production image metadata reports 43,407,177 bytes (about
41.4 MiB); it runs as uid 10001 with entrypoint `braindrop-cli` and command
`start --all`. Native binary sizes were: Linux glibc AMD64 21,248,752 bytes,
ARM64 19,118,992; Linux musl AMD64 21,567,888, ARM64 17,865,096; macOS ARM64
17,867,696, AMD64 19,106,088. All six native archives were repackaged from the
binaries built after implementation commit `65f3972`; package `RELEASE` records
that source revision. Archive sizes and SHA-256 values:

| Archive | Size | SHA-256 |
|---|---:|---|
| `braindrop-linux-amd64-glibc.tar.gz` | 8,822,864 bytes | `efd10263c0fa13a44658291e3d7e660d80d28bf99ecd83ad9c57eb5ad5aeaaf7` |
| `braindrop-linux-arm64-glibc.tar.gz` | 8,346,639 bytes | `4a814fce34b4f305d609721c763bd4d5315b7fb1d5c8c7ef861319cb86771718` |
| `braindrop-linux-amd64-musl.tar.gz` | 9,040,096 bytes | `777562ffe91a9f35b59eb96b339a92fe930cc6b51a7f076fb8e79c30a530283b` |
| `braindrop-linux-arm64-musl.tar.gz` | 8,310,047 bytes | `6b205923de8241ebfb288583d5a040960bdd1389d6763a394bc09631e541b33b` |
| `braindrop-macos-arm64.tar.gz` | 8,030,822 bytes | `f75a3826fc664b66e60f12897442f4daf6b8c1099d28d20a0d67296375c442aa` |
| `braindrop-macos-amd64.tar.gz` | 8,564,653 bytes | `06cd3efb3612ff99a93a71b76875bea89c24c52e8e790e9562ebc88d031fe9a7` |

All six sidecars passed `shasum -a 256 -c`; all six package layout checks
passed. The archive outputs remain ignored local build artifacts and were not
committed or published.

Native process RSS, container memory and total VPS memory are distinct measures.
Fly startup/health and idle process RSS plus database file sizes are measured;
TierHive app RSS, both platforms’ active/peak usage, CPU, authenticated endpoint
latency, sustained workload, database growth and background-job overhead remain
unmeasured. See the benchmark document for the Task 009 methodology limits
(no pre-remediation baseline, no separately instrumented pool acquisition
latency, and no seeded queue jobs).

## Security and public demo assessment

Manual capture requires no AI key. Public-demo settings keep registration off,
do not set a server-wide `OPENROUTER_API_KEY`, and rely on existing encrypted
BYOK and write-only credential responses. Authentication and AI limits are
process-local; item creation has no quota; database growth and demo retention
are unbounded; no automatic account/data cleanup exists. The conservative
recommendation is to defer a public demo until an operator can handle account
provisioning, synthetic-data messaging, retention and support. No demo subsystem
was added.

## CI, tests and dependency audit

- `./scripts/ci.sh`: **PASS** — 101 Rust unit tests, 103 Rust integration/model/
  task tests, 29 TypeScript binding drift tests, 36 frontend unit tests, 14
  Playwright tests passed and one screenshot test skipped. Clippy/fmt, SQLite
  migrations, backup/restore, production Docker health, worker/scheduler
  startup, persistence restart, static assets and browser security checks passed.
- Local TOML parse, Make help and target dry-runs: **PASS**.
- `scripts/typescript-bindings.sh check`: included in CI and **PASS**.
- Docker volume/non-root runtime: production CI passed; local constrained
  container verified uid 10001 and bind mount ownership. Fly volume ownership
  and process UID were also **PASS** on the remote machine.
- `scripts/test-backup-restore.sh`: CI **PASS** for integrity, queue-first
  ordering and linked pending delivery/job restore. Fly backup/restore also
  passed using disposable DB copies, but not pending notification boundary data.
- `cargo audit --no-fetch`: no known vulnerabilities; three allowed upstream
  unmaintained warnings remain for chrono 0.4.45, chrono-tz 0.10.4 and
  unsafe-libyaml 0.2.11, with no fixed upgrade. Advisory DB refresh could not
  acquire the Cargo index lock; it used the locally installed advisory DB.
- `pnpm audit --audit-level=moderate`: retried with project-pinned pnpm 10.12.4;
  **BLOCKED** because npm registry DNS resolution failed (`ENOTFOUND`). No
  dependency changed in Task 010.
- Fly strict configuration validation: **PASS**. HTTPS, health, frontend, volume,
  restart and explicit-stop cold-start checks passed as described above.
- All six native target binaries rebuilt successfully after the documentation
  and configuration changes; all six archives were repackaged and checksum /
  layout-verified as recorded above.

## Changed files

`fly.toml`, `Makefile`, `.env.example`, `README.md`, `docs/ARCHITECTURE.md`,
`docs/DEPLOYMENT.md`, `docs/PROGRESS.md`, `docs/BENCHMARK.md`, and this report.
Task009 runtime code, auth/API contracts, SQLite tuning and regression tests
were preserved.

## Remaining limitations and release readiness

Fly HTTPS/health, volume, UID, restart, cold-start, and disposable-copy
backup/restore passed. Authenticated item behavior, active workload, exact
auto-stop delay, and notification delivery were not validated. TierHive Alpine inspection and
staging passed, but application startup failed; service logs require explicit
operator approval before their potentially sensitive contents can be read. The
managed proxy backend/DNS issue also remains. pnpm audit could not reach npm.
The public demo is not approved for publication and its URL is not in README.
Native archives were previously checksum-verified; this turn changed no native
runtime/package contents.

**Fly deployment: RUNNING, PARTIALLY VALIDATED.**
**TierHive deployment: BLOCKED AT STARTUP AND DNS/PROXY VALIDATION.**
**Task 010: INCOMPLETE WITH REMOTE VALIDATION LIMITATIONS.**
No pushes, tags, GitHub Actions, GitHub releases or public image publications
were made.
