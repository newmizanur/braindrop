# Task 010 — Deployment validation record

**Date:** 2026-11-19  
**Baseline:** Task 009 commit `103c591969b52f93c8a5254f50edfc5ca23105e6`  
**Scope:** Fly.io deployment preparation, TierHive deployment readiness and
benchmark documentation. No remote host or Fly resource was contacted or
changed, no real secret was configured, and no public demo URL was published.

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

## Fly.io configuration and validation

`fly.toml` uses the checked-in `Dockerfile`, whose entrypoint and command run
`braindrop-cli start --all` as uid 10001. Existing config uses 5150, the
`/_health` endpoint, `FlyClientIp` for client IP trust behind Fly Proxy, and
SQLite app/queue URLs beneath `/data`. Demo autostop uses `stop` / autostart /
zero minimum running machines. Fly's proxy checks excess capacity every few
minutes; a 15-minute stop interval is not configurable. Always-on settings are
documented for scheduled production use.

The file parsed with Python `tomllib`. Fly CLI `config validate --strict` and
`config show --local` could not run because flyctl requires an authenticated
access token for validation in this environment. No login or Fly API action was
attempted. Remote mount ownership, startup, machine count, health routing,
volume persistence, stop/start and upgrade behavior are **UNTESTED** pending
authorization. Dockerfile's `/data` ownership is uid 10001; the Fly mount's
effective ownership must be confirmed on a real machine.

Fly pricing documentation was checked 2026-11-19. The published lowest-region
256 MiB shared-cpu-1x reference is $2.19 per 30 days; Singapore prices vary and
the exact 256 MiB quote should be checked in Fly's calculator. A 1 GiB volume
reference is $0.15/month, Asia-Pacific egress $0.04/GB and snapshots $0.08 per
stored GB-month after the first 10 GB free. This is not a fixed total estimate.
See [Fly pricing](https://fly.io/docs/about/pricing/) and [volume storage](https://fly.io/volumes/).

## Make targets

`make help` lists all six Fly targets. `make -n fly-init` and
`make -n fly-deploy` rendered expected commands. flyctl v0.5.13 help confirmed
the used `volumes create`, `machine list --quiet`, `deploy --ha=false`,
`scale memory`, `apps restart`, `logs` and `ssh console` command forms. The
targets were **NOT run** against a Fly account. The documented Fly CLI install
and `fly auth login` are prerequisites; provisioning may incur charges.

## TierHive results

**BLOCKED pending explicit authorization to connect.** The authorized access
boundary requires prior approval before SSH to the supplied TierHive host. No
connection was attempted. Alpine version, architecture, RAM/swap, disk,
OpenRC/services/ports, TLS proxy, package availability, deployment, service
startup, administrator flow, persistence, backups, restore, upgrade/rollback,
and restart recovery remain **UNTESTED**. The host address and SSH port are not
stored in this repository.

## Benchmark and resource measurements

The Task 009 pool results are preserved in `docs/BENCHMARK.md` without a
pre/post claim. No TierHive or Fly server specifications or metrics are
available because neither remote system was inspected or deployed.

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
TierHive/Fly startup, idle/active/peak RSS, CPU, endpoint latency, database
growth, and worker/scheduler overhead remain unmeasured. See the benchmark
document for prior methodology and its limits (unrecorded host specs, no
pre-remediation baseline, no separately instrumented pool acquisition latency,
and no seeded queue jobs).

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
  container verified uid 10001 and bind mount ownership. Fly volume mount itself
  remains **UNTESTED**.
- `scripts/test-backup-restore.sh`: CI **PASS** for integrity, queue-first
  ordering and linked pending delivery/job restore. It does not pause a live
  scheduler at the precise enqueue boundary; no Fly snapshot/restore was run.
- `cargo audit --no-fetch`: no known vulnerabilities; three allowed upstream
  unmaintained warnings remain for chrono 0.4.45, chrono-tz 0.10.4 and
  unsafe-libyaml 0.2.11, with no fixed upgrade. Advisory DB refresh could not
  acquire the Cargo index lock; it used the locally installed advisory DB.
- `pnpm audit --audit-level=moderate`: **BLOCKED** because npm registry access
  failed DNS resolution (`ENOTFOUND`). No dependency changed in Task 010.
- Fly CLI strict configuration validation: **BLOCKED** without an authenticated
  Fly token. It was not possible to assert machine-volume behavior locally.
- All six native target binaries rebuilt successfully after the documentation
  and configuration changes; all six archives were repackaged and checksum /
  layout-verified as recorded above.

## Changed files

`fly.toml`, `Makefile`, `.env.example`, `README.md`, `docs/ARCHITECTURE.md`,
`docs/DEPLOYMENT.md`, `docs/PROGRESS.md`, `docs/BENCHMARK.md`, and this report.
Task009 runtime code, auth/API contracts, SQLite tuning and regression tests
were preserved.

## Remaining limitations and release readiness

Remote Fly config/API validation, volume ownership and persistence, Fly startup,
scheduled delivery after cold start, TierHive inspection/deployment, 256 MiB
and sustained 128 MiB capacity, native service execution on Alpine, Ubuntu and
macOS, and public demo operation are not fully validated here. Native packages
were rebuilt and checksummed, but these checks do not replace on-target runtime
verification. A public demo URL is not recommended or included.

**Local deployment preparation: READY FOR AUTHORIZED REMOTE VALIDATION.**
**Task 010 overall: INCOMPLETE WITH REMOTE VALIDATION BLOCKED PENDING APPROVAL.**
No code was pushed, tagged, published, deployed, or sent to GitHub Actions.
