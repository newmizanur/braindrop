# Task 009 — Independent Code Review Remediation

**Date:** 2026-11-19
**Baseline:** `docs/tasks/008-code-review.md`
**Scope:** Remediation and final local validation for BrainDrop v0.1.0. No
publishing, tagging, deployment, push, or hosted workflow was performed.

## Executive summary

All confirmed actionable findings from Task 008 were addressed. Docker-first CI
passed on the final code state, including 204 Rust tests, 36 frontend tests,
production startup/persistence/backup checks, and 14 Playwright tests (one
release screenshot test is intentionally skipped). Six native target binaries
were rebuilt and packaged; all six SHA-256 sidecars verified. Dependency audits
found no known vulnerabilities, with three upstream unmaintained advisories
retained because no fixed upgrade exists and the project requires retaining the
current date/time/framework dependencies absent a justified replacement.

The benchmark supports application pool maximum 4 as a sensible lightweight
default: it reached effectively the same throughput as 10 with lower tail
latency and lower RSS. The results are synthetic, from one host, and not a
capacity guarantee. There is no comparable pre-remediation benchmark, so no
before/after performance claim is made.

## Task 008 finding disposition

| Finding | Status | Remediation/evidence |
|---|---|---|
| 008-01 Atomic magic-link use | FIXED | Conditional token consume update; concurrent integration regression permits one winner. |
| 008-02 Login rate order/password bounds | FIXED | Client budget precedes lookup/hash; account failure budget and dummy hash retained; registration/reset enforce 8–128 chars; login permits legacy short passwords but caps at 128. Concurrent request tests and Docker auth tests pass. |
| 008-03 Registration race/transaction | FIXED | Hash before insert, database unique constraint resolves races; concurrent duplicate test passes. |
| 008-04 SQLite pools | FIXED | App min/max 1/4, queue max 2/workers 2 in dev/prod/test configs. Benchmark below. |
| 008-05 Notification transient preparation failure | FIXED | Bounded retry policy and regression test verifies retry state after transient DB failure. |
| 008-06 Cross-file backup ordering | FIXED | Queue-first online backups, documented non-atomic snapshots and quiesced procedure; backup/restore relation test and Docker backup pass. |
| 008-07 DTO/TS contract drift | FIXED | `TS_RS_EXPORT_DIR`, generation/check script in CI, strict enum unions, optional/null semantics, generated API inputs. 29 drift tests pass. |
| 008-08 Tag truncation/validation mismatch | FIXED | Shared normalized tag validation rejects more than 50; DTO tests cover create/update. |
| 008-09 Dashboard query count | FIXED | Single conditional aggregate query; timezone boundary integration tests pass. |
| 008-10 Persisted enum fallback | FIXED | Fallible DTO conversion logs and reports invalid persisted enum values as internal errors. |
| 008-11 Reqwest client construction | FIXED | Shared timeout-keyed clients, per-request credentials preserved. |
| 008-12 No-op downloader worker | FIXED | Scaffold module and registration removed. |
| 008-13 Redundant item index | FIXED | Reversible migration removes standalone user index; `EXPLAIN QUERY PLAN` uses covering composite index. |
| 008-14 Symlink activation | FIXED | Same-directory temporary symlink and `mv -Tf` for activation and rollback; replacement checked in Alpine BusyBox 3.20 and Debian GNU coreutils. |
| 008-15 Unmaintained Rust crates | DEFERRED | `cargo audit` reports no fixed upgrade for chrono 0.4.45, chrono-tz 0.10.4, unsafe-libyaml 0.2.11. No replacements introduced solely to silence advisories. |
| 008-16 Direct axum-extra dependency | FIXED | Removed direct dependency; it remains required transitively by Loco. |
| Responsive/accessibility test coverage | DEFERRED | No new accessibility/mobile-specific suite was added; existing browser smoke suite remains. |
| Pre/post remediation benchmark comparison | DEFERRED | No old-revision baseline was captured. Current pool-size comparison is recorded below. |

## SQLite pool benchmark

Run using `scripts/benchmark-sqlite-pools.py`: production config, app and queue
on separate temporary SQLite files, `start --all` with worker/scheduler running,
250 seeded items, 12 concurrent clients, 20 seconds per pool size, mix of 55%
item-list reads, 30% stats reads, 15% item writes. Each request was timed. RSS
was sampled from the process after each workload; SQLite busy and connection
pool acquisition timeout log/error counts were zero in all samples. Acquisition
latency itself was not separately instrumented.

| App max connections | Requests | Throughput | p50 | p95 | p99 | Sampled RSS | Busy/timeouts |
|---:|---:|---:|---:|---:|---:|---:|---:|
| 1 | 14,855 | 742.0 req/s | 15.63 ms | 21.92 ms | 24.48 ms | 38.4 MB | 0 / 0 |
| 2 | 24,078 | 1,202.9 req/s | 9.41 ms | 15.28 ms | 18.65 ms | 39.7 MB | 0 / 0 |
| 4 | 30,059 | 1,501.9 req/s | 7.41 ms | 12.70 ms | 16.58 ms | 37.0 MB | 0 / 0 |
| 10 | 30,200 | 1,508.7 req/s | 7.14 ms | 14.70 ms | 22.66 ms | 50.3 MB | 0 / 0 |

Pool size 4 is the default: 10 achieved only 0.5% more throughput while p95
latency was 2.0 ms higher, p99 was 6.1 ms higher, and sampled process RSS was
13.3 MB higher. Pool size 1 remains configurable for memory-constrained hosts;
size 2 is an intermediate option. The benchmark does not model a busy production
worker queue (workers were running but idle because no jobs were seeded), and
connection acquisition latency was not instrumented separately. Those results
are not release-blocking for the intended small single-replica deployment but
are limitations of the evidence.

SQLite retained Loco's WAL, `synchronous=NORMAL`, 5-second busy timeout, and
foreign keys enabled per connection. Dashboard query plan after migration
reports `SEARCH items USING COVERING INDEX idx_items_user_category (user_id=?)`;
the redundant standalone `idx_items_user_id` index is absent.

## Validation and artifacts

- `./scripts/ci.sh`: PASS. Rust fmt + Clippy; 101 library tests and 103
  integration/model/task tests; 29 binding drift tests; SQLite migrations;
  frontend typecheck/lint/build and 36 frontend unit tests; production image
  health and worker/scheduler startup; persistence through restart; in-container
  backup; 14 Playwright tests passed, one screenshot capture skipped.
- `cargo audit --no-fetch`: PASS with three allowed unmaintained warnings and
  no known vulnerabilities. `pnpm audit --audit-level=moderate`: no known
  vulnerabilities.
- `scripts/typescript-bindings.sh check`: PASS (29 tests).
- `scripts/test-backup-restore.sh`: PASS; relational fixture confirms queue-first
  ordering and linked pending delivery/job restoration. Production Docker backup
  also passed in CI.
- Atomic symlink replacement check: PASS in Alpine BusyBox and Debian GNU.
- Native release binaries rebuilt: Linux AMD64/ARM64 glibc, Linux AMD64/ARM64
  musl, macOS ARM64/AMD64. Package layout checks passed; all six archive
  sidecars verified with `shasum -a 256 -c`.
- Docker production startup: PASS through CI health, worker/scheduler startup,
  persistence and static asset checks.
- No target hardware/runtime smoke test was performed for the six standalone
  native binaries. Cross-target compilation and package verification are not a
  substitute for executing them on each operating system/architecture.

## Simplification and dependency review

- Removed unused direct `axum-extra` dependency (Loco still brings it transitively).
- Removed the no-op downloader worker and registration.
- Consolidated the dashboard's 14 count queries into one aggregate statement.
- Reused Reqwest clients by timeout configuration rather than rebuilding them.
- Retained Loco/SeaORM and chrono/chrono-tz. The audit supplies no fixed upgrade
  for the three unmaintained warnings; no alternative was shown to be a
  compatible, lower-risk change for this release.
- No product feature or broad architecture rewrite was added.

## Remaining limitations

- Sequential online backups of separate application and queue SQLite files are
  not an atomic cross-file snapshot. Queue-first ordering and recovery reduce
  orphan risk; stopping the service before backup is documented for stronger
  consistency.
- Rate-limit state is process-local and resets on restart; deployment remains
  single-replica.
- Browser JWT remains in local storage with CSP as a mitigation, not elimination,
  of XSS token theft risk.
- Native target runtime verification was not possible on every platform here.
- Performance numbers are a local synthetic comparison, not a pre/post baseline
  or sustained-capacity claim.

## Release readiness

**READY WITH LIMITATIONS.** Task 009 implementation and the requested local
validation are complete. No release action was performed. Tagging, publishing,
pushing, and deployment remain unauthorized and were not attempted.

## Finding-level remediation records

The locations below point to the final implementation; the original review
locations are listed in `docs/tasks/008-code-review.md`. Complexity is an
estimate of the remediation performed, not an estimate of future maintenance.

| Finding / severity | Final evidence | Impact addressed | Fix and complexity |
|---|---|---|---|
| 008-01 Medium | `src/models/users.rs:86-106`; `src/controllers/auth.rs:324-345` | Prevents concurrent redemption of one magic link more than once. | Conditional consume update; **M**. |
| 008-02 Medium | `src/controllers/auth.rs:205-227`; `src/auth_rate_limit.rs:152-188`; `src/models/users.rs:16-38` | Caps pre-authentication password work before Argon2; preserves account failure controls, timing parity, and legacy login compatibility. | Split client/account budgets and bounded DTOs; **M**. |
| 008-03 Medium | `src/models/users.rs:288-312`; `tests/models/users.rs:71-90` | Removes expensive hashing from DB transaction and resolves registration races through DB uniqueness. | Hash then insert and map unique constraint; **S**. |
| 008-04 Medium | `config/{development,production,test}.yaml` (app pool/queue sections); `.env.example:27` | Limits SQLite connection contention and idle resources; keeps worker concurrency bounded. | Explicit Loco-supported pool config; **S**. |
| 008-05 Medium | `src/notifications/scan.rs:282-355`; `tests/requests/notifications.rs` transient-preparation regression | Avoids losing notification deliveries on temporary DB/connection failures. | Existing bounded retry/backoff extended to preparation; **M**. |
| 008-06 Medium | `scripts/backup.sh:72-84`; `docs/DEPLOYMENT.md` backup procedure; `scripts/test-backup-restore.sh` | Prevents restored queued jobs without their application delivery rows in the persist-before-enqueue sequence. | Queue-first order, recovery note, quiesced procedure; **S**. |
| 008-07 Medium | `src/dtos/items.rs:15-35, 39-80`; `scripts/typescript-bindings.sh:7-16`; `frontend/src/api/items.ts` | Prevents TS clients accepting invalid enums or treating missing/null PATCH fields incorrectly; catches drift in CI. | Central export env, generated aliases, drift check; **M**. |
| 008-08 Low | `src/models/items.rs:250-271`; `src/dtos/items.rs:118-124, 242-249` | Prevents oversized tag lists silently losing user data and aligns create/update. | Shared validation after normalization; **S**. |
| 008-09 Low | `src/models/stats.rs:74-115` | Removes 14 separate aggregate round trips while preserving timezone boundaries. | One conditional aggregate query; **M**. |
| 008-10 Low | `src/dtos/items.rs:39-80`; `src/controllers/items.rs:38-102` | Prevents corrupt/unknown persisted enums being rendered as plausible defaults. | Fallible conversion with logged internal error; **S**. |
| 008-11 Low | `src/ai/client.rs:23-42`; `src/notifications/provider.rs:327-345` | Reuses connection pools while ensuring user credentials remain request-scoped. | Timeout-keyed shared Reqwest clients; **S**. |
| 008-12 Low | `src/app.rs:19-23, 78-83`; removed `src/workers/downloader.rs` | Removes a no-op scaffold worker from runtime registration. | Delete unused worker/module registration; **S**. |
| 008-13 Low | `migration/src/m20261119_000003_drop_redundant_item_user_index.rs:6-24` | Avoids redundant index write/storage cost while retaining a covering composite index for the query plan. | Reversible migration; **S**. |
| 008-14 Low | `scripts/deploy.sh:247-257, 394-402, 488-492` | Prevents a deployment/rollback interruption from exposing an absent `current` link. | Same-directory temporary link + rename; **S**. |
| 008-15 Low / maintenance | `Cargo.lock`; final `cargo audit --no-fetch` output in this report | Avoids destabilizing date/time and framework dependencies solely to silence maintenance notices; no known vulnerability is reported. | Retain until a compatible maintained upgrade exists; **N/A** for code change. |
| 008-16 Low | `Cargo.toml:15-35`; `Cargo.lock` | Removes an unnecessary direct dependency declaration without removing Loco's transitive use. | Remove direct dependency; **S**. |

Statuses for deferred coverage/evidence (not code defects): accessibility/mobile
testing and a pre-remediation benchmark baseline are **DEFERRED** as described
above. The per-target native runtime checks were not performed and are recorded
as a limitation, not a claimed pass.
