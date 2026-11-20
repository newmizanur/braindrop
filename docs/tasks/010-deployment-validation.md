# Task 010 — Deployment validation record

**Date:** 2026-11-19 to 2026-11-20
**Baseline:** Task 009 commit `103c591969b52f93c8a5254f50edfc5ca23105e6`  
**Scope:** Local Task 010 preparation plus authorized TierHive inspection and
Fly.io deployment validation. No repository contains remote addresses, SSH
credentials, encryption keys or administrator passwords. A publicly reachable
Fly deployment exists, but no demo URL was added to README and no credentials
were published.

## Table of Contents

<!-- toc:start -->
- [Implementation summary](#implementation-summary)
- [Fly.io configuration and validation](#flyio-configuration-and-validation)
- [Make targets](#make-targets)
- [TierHive results](#tierhive-results)
- [Benchmark and resource measurements](#benchmark-and-resource-measurements)
- [Security and public demo assessment](#security-and-public-demo-assessment)
- [CI, tests and dependency audit](#ci-tests-and-dependency-audit)
- [Changed files](#changed-files)
- [Remaining limitations at initial Task 010 close (superseded below)](#remaining-limitations-at-initial-task-010-close-superseded-below)
- [2026-11-20 continuation: native runtime and remote recheck](#2026-11-20-continuation-native-runtime-and-remote-recheck)
- [Task 011 addendum — public HTTPS check (2026-11-20)](#task-011-addendum-public-https-check-2026-11-20)
<!-- toc:end -->
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

The deployed production image reported 37 MB and runs as uid 10001 with command
`start --all`. The six native archives were packaged from binaries built at
commit `b329cba`; each `RELEASE` file records that revision. Archive sizes
and SHA-256 values:

| Archive | Size | SHA-256 |
|---|---:|---|
| `braindrop-linux-amd64-glibc.tar.gz` | 8,824,539 bytes | `e233f762da8ca3886d15e998b27649ea469ec7c4a63eab93ec992c5295f6769b` |
| `braindrop-linux-arm64-glibc.tar.gz` | 8,350,127 bytes | `2fe652f4b238493feb3dcf21b1aa1aa83f74b43da3cc3c07615fc29d6ce14e99` |
| `braindrop-linux-amd64-musl.tar.gz` | 9,040,757 bytes | `d6d8234590b2c223f296527c5da44f1bd8316bceb76bf70ce0023c1f802b336d` |
| `braindrop-linux-arm64-musl.tar.gz` | 8,311,460 bytes | `f2533a1a3b8df0b19fdcd7b3fdc133b887dadb992220aa2e3c062a6a25fe850e` |
| `braindrop-macos-arm64.tar.gz` | 8,030,767 bytes | `cb20066c0e2b6f891eb6441962548b75ab86815e5fde30b660b993cf4e9c9888` |
| `braindrop-macos-amd64.tar.gz` | 8,569,024 bytes | `37f967bf47eab4682b43281ff9cb577cbf3ba46abaf2763d9da3dc11b67c39de` |

All six sidecars passed `shasum -a 256 -c`; all six package layout checks
passed. The archive outputs remain ignored local build artifacts and were not
committed or published.

Native process RSS, container memory and total VPS memory are distinct measures.
See docs/BENCHMARK.md for post-deployment process samples and a small Fly
authenticated workload. Sustained workload, cgroup peak memory, TierHive
authenticated latency, CPU utilization and remote notification recovery remain
unmeasured. Task 009 had no pre-remediation baseline, separately instrumented
pool acquisition latency or seeded queue jobs.

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

- `./scripts/ci.sh`: **PASS** — 104 Rust unit tests, 103 Rust integration/model/
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

Task 010 changes are in the root `fly.toml`, `Dockerfile`, `Makefile`,
`.env.example`, `README.md`, `docs/DEPLOYMENT.md`, `docs/PROGRESS.md`,
`docs/BENCHMARK.md`, and this report. Native deployment changes are in
`scripts/deploy.sh`,
`scripts/create-runtime-archive.sh`, `scripts/prepare-release.sh`,
`scripts/migrate-native-layout.sh`, `scripts/deploy-init-alpine.sh`,
`scripts/deploy-init-ubuntu.sh`, and `scripts/package.sh`. Regression tests
are in `scripts/test-package-deploy.sh`, `scripts/test-native-layout-migration.sh`,
and `scripts/ci-rust.sh`. The only application-code change is the existing
admin CLI's secure stdin password option and its tests. Task 009 auth/API
contracts, SQLite pool tuning and TypeScript bindings were preserved.

## Remaining limitations at initial Task 010 close (superseded below)

This section records the initial Task 010 report before the later authorized
TierHive remediation and revalidation. It is superseded by the dated
continuations below. At that point, public TLS and startup were unresolved.

## 2026-11-20 continuation: native runtime and remote recheck

The owner reported and fixed the original TierHive start failure: the binary
was executable as root but not by the non-root service account because the
release tree retained restrictive/non-root ownership and archive metadata. The
owner normalized release ownership to root and directories/binary to executable
readable modes. A read-only recheck now confirms OpenRC is started and enabled
at boot, the app process runs as UID 103, the binary is root-owned mode 755,
`.env` is service-owned mode 600, and local health, the frontend and a SPA deep
link all return 200. Both databases pass integrity checks. This recheck did not
read production environment values or logs.

At this intermediate check, the public endpoint's HTTPS request failed during
TLS negotiation. The owner later reported that Cloudflare HTTPS was restored;
the final public check is recorded in the Task 011 addendum below. No
authentication credentials were sent over public HTTP. The managed proxy/DNS
was not changed by this work.

Local changes now prepare a flat `/opt/braindrop` runtime, keep service
definitions under `/etc`, normalize package ownership/modes, exclude macOS
metadata, separate public distribution archives from runtime upload archives,
and preserve existing databases, backups, logs and `.env` while migrating the
former `shared/` layout. Upgrades use one code-only previous-runtime archive;
they take a DB backup before migrations and restore old code on failed health
checks. Database schema rollback still requires restoring the matching DB
backup. Cleanup refuses to remove old release trees containing `.env`, SQLite
files or state directories. These script changes are local and have not yet
been deployed to TierHive.

`admin:create` now supports `password_stdin:true` to avoid putting production
passwords in process arguments, with password length and stdin parsing tests.
Separate TierHive and Fly administrator accounts, authenticated workload
measurements and notification recovery checks remain pending the operator's
email and display name.

The running pre-migration TierHive process sample was 22,476 KiB RSS, 24,840
KiB process high-water RSS, with 45,692 KiB host memory available and 130,072
KiB of 131,068 KiB swap free. App and queue databases occupied 160 KiB. Three
local health samples were 0.7–14.2 ms; frontend and deep-link checks were 1.2
and 0.7 ms. CPU, active workload, startup time and peak-under-load were not
measured. These idle observations do not establish capacity on the host.

Host fallback checks passed: formatting and Clippy; Rust tests (104 unit, 103
integration/model/task); frontend typecheck, lint, unit tests (36), build and
TypeScript binding drift; and Playwright (14 passed, one screenshot test
skipped). The Docker engine was accessible outside the restricted sandbox.
After correcting test fixture issues, full `./scripts/ci.sh` passed, including
Linux archive/ownership/non-root tests, production image health, workers,
scheduler, persistence, backup and Playwright checks. All six release targets
were rebuilt and packaged from the updated source tree; checksum and archive
metadata verification is recorded in the continuation report after commit.
`pnpm audit` was retried but npm registry DNS returned `ENOTFOUND`. Cargo audit
used the local advisory database and reported the already-documented
unmaintained `chrono`, `chrono-tz`, and `unsafe-libyaml` advisories with no
fixed upgrades.

The current Fly machine is stopped by its configured auto-stop behavior and
remains at the existing 256 MiB/one-volume shape. Strict Fly config validation
passes. Account authentication, item persistence after cold start, active
resource measurements and notification job recovery have not yet been repeated
in this continuation.

**Superseding final remote validation (2026-11-20):** the paragraph above
records an intermediate state and is superseded by this section. The flat
native runtime is deployed on TierHive. It runs as UID 103; OpenRC reports
started and enabled at boot; health, frontend and SPA deep link return 200;
both SQLite databases pass integrity checks. TierHive's app+queue directory
occupies 172 KiB. The process sample is 17,492 KiB RSS and 18,236 KiB HWM; host
available RAM is 50,228 KiB and free swap 130,072 KiB. The runtime was
inventoried without reading secret values. Backup/restore against disposable
databases, deployment upgrade, previous-runtime rollback, redeployment and
OpenRC restart passed. Full host reboot and TierHive authenticated item
operations were not performed.

The Fly machine is started and remains exactly one shared CPU/256 MiB machine
in Singapore with its existing 1 GiB volume and auto-stop/autostart/minimum-zero
settings. Current health passes. Authenticated create/read, restart persistence
and encrypted BYOK settings passed using a transient test account; all test
records and the test key were removed. The process sample is UID 10001,
23,984 KiB RSS and 41,212 KiB HWM; /data occupies 232 KiB including the
lost+found directory; both databases pass integrity checks. Actual native
idle-stop timing and remote pending-notification recovery were not tested.

Final TierHive public HTTPS validation is recorded in the Task 011 addendum:
Cloudflare returned HTTP 200 for health, SPA and static assets. The request path
is working, but the Cloudflare SSL/TLS mode and encryption on the
Cloudflare-to-origin connection could not be read from public responses. The
configured app client-IP source is RightmostXForwardedFor; the managed
Cloudflare/HAProxy forwarding and overwrite behavior was not independently
observable. Permanent administrator creation on either platform was deferred
because no identity details were supplied for the earlier request.

The report contains the Task 009 historical pool results and separate
post-deployment observations. Resource samples are too small to substantiate
capacity or reliability at 128/256 MiB. pnpm audit remains blocked by npm
registry DNS resolution; Cargo reports the three documented upstream
unmaintained advisories with no fixed versions. CI passed on the source revision
recorded above, and all six package checksum/layout validations passed.

**Task 010: READY WITH LIMITATIONS.** TierHive and Fly are operating. TierHive
public HTTPS and health are now validated through Cloudflare, but origin-leg
encryption, proxy client-IP forwarding, authenticated workflows, full reboot,
remote notification recovery, sustained/peak resource behavior and exact Fly
auto-stop timing remain unvalidated. No push, tag, release, image publication
or GitHub Actions run occurred.

## Task 011 addendum — public HTTPS check (2026-11-20)

Without credentials and without changing Cloudflare, DNS or HAProxy settings,
read-only curl requests through the public edge returned HTTP 200 for the
health endpoint, root page, an SPA deep route and a compiled JavaScript asset.
Responses carried Cloudflare edge headers, BrainDrop security headers and
Cache-Control no-store on health/API responses. GET /api/auth/config returned
allow_registration:false; an unauthenticated GET /api/items returned 401 and
no Access-Control-Allow-Origin for an untrusted origin. An OPTIONS preflight
received 405 and no CORS allow headers. These checks confirm public routing,
disabled registration and unauthenticated access controls; they do not validate
successful login or authorized-user CORS behavior.

The public browser-to-Cloudflare HTTPS leg is verified. The response did not
include HSTS. Cloudflare documents that Flexible mode leaves the origin leg
unencrypted, while Full and Full (strict) use HTTPS to the origin; the zone
setting itself is not visible in public responses, so Cloudflare-to-HAProxy
encryption remains unknown
([Cloudflare SSL/TLS modes](https://developers.cloudflare.com/ssl/origin-configuration/ssl-modes/)).
The app's configured REMOTE_IP_SOURCE is RightmostXForwardedFor, but the
managed Cloudflare and HAProxy forwarding chain could not be inspected, so
preservation and trust of the real client address remain unverified. A Python
standard-library probe received a Cloudflare 1010 block; curl succeeded. This
was a client fingerprinting difference, not an application failure.
