# Task 008 — Independent Code Review

**Review date:** 2026-11-19
**Scope:** BrainDrop v0.1.0 source, configuration, frontend, deployment artifacts,
tests, bundled Loco 1.3.1 guidance, and dependency advisories. This was a
read-only review; CI was not rerun during the review.

## Table of Contents

<!-- toc:start -->
- [Executive assessment](#executive-assessment)
- [Findings recorded for Task 009](#findings-recorded-for-task-009)
- [Other review observations](#other-review-observations)
- [Release recommendation at review time](#release-recommendation-at-review-time)
<!-- toc:end -->
## Executive assessment

The application follows Loco conventions and is appropriate for a lightweight,
single-replica self-hosted product. No confirmed critical or high-severity issue
was found. The reviewer recommended addressing authentication races and
resource-abuse paths, notification retry handling, pool sizing, cross-file
backup ordering, and generated TypeScript contract drift before release.

## Findings recorded for Task 009

| ID | Severity | Finding | Evidence at review | Task 009 status |
|---|---|---|---|---|
| 008-01 | Medium | Magic-link read/clear was not atomic; concurrent verification could issue multiple sessions. | `src/controllers/auth.rs` read token then called `clear_magic_link`. | Fixed — conditional single-row token consumption and concurrency regression test. |
| 008-02 | Medium | Login rate check happened after Argon verification; password length had no upper bound. | `src/controllers/auth.rs` and `src/models/users.rs` login DTO. | Fixed — client budget before lookup/hash, retained dummy hash and failure-only account budget; registration/reset enforce 8–128 characters and login caps at 128 while permitting legacy shorter passwords. |
| 008-03 | Medium | Registration checked for duplicates and hashed inside a transaction. | `src/models/users.rs` `create_with_password`. | Fixed — hash before insert, rely on unique constraint and classify unique-key errors. |
| 008-04 | Medium | App pool default was 10 while SQLite queue had independent framework defaults. | `config/production.yaml`, `config/development.yaml`, Loco 1.3.1 queue config. | Fixed — app 1/4 and queue max 2/workers 2; benchmarked 1, 2, 4, 10. |
| 008-05 | Medium | Preparation/database errors for notifications were terminal instead of retried. | `src/notifications/scan.rs` `deliver`. | Fixed — retryable preparation failures use bounded existing backoff; regression test drops the settings table after claim. |
| 008-06 | Medium | Sequential app-then-queue backups could restore an orphan queue job. | `scripts/backup.sh`; `src/notifications/scan.rs` persists delivery before enqueue. | Fixed — queue-first ordering, explicit non-atomic caveat, quiesced procedure, linked-fixture backup/restore test. |
| 008-07 | Medium | Generated request bindings weakened enums and create optionality; frontend duplicated the create/update types without drift checking. | `src/dtos/items.rs`, `frontend/src/bindings/`, `frontend/src/api/items.ts`. | Fixed — centralized TS_RS_EXPORT_DIR script, strict enum unions, optional/nullable shape corrections, generated API types and CI drift check. |
| 008-08 | Low | Tag normalization truncated beyond 50 before validation; create/update differed. | `src/models/items.rs`, `src/dtos/items.rs`. | Fixed — normalize, then shared create/update count and length validation rejects excessive tags. |
| 008-09 | Low | Dashboard computed 14 separate count queries. | `src/models/stats.rs`. | Fixed — one conditional aggregate scan; existing timezone boundaries retained and integration-covered. |
| 008-10 | Low | Response DTO conversion silently defaulted unknown persisted enums. | `src/dtos/items.rs`. | Fixed — fallible conversion logs invalid persisted values and returns an internal error. |
| 008-11 | Low | Reqwest clients were rebuilt per provider operation. | `src/ai/client.rs`, `src/notifications/provider.rs`. | Fixed — timeout-keyed shared clients; keys and webhook credentials remain request-scoped. |
| 008-12 | Low | No-op scaffold downloader remained registered. | `src/workers/downloader.rs`, `src/app.rs`. | Fixed — worker and registration removed. |
| 008-13 | Low | Standalone `user_id` index duplicated the leading key of composite item indexes. | `migration/src/m20261118_145040_items.rs`. | Fixed — new reversible migration drops the index; query plan still uses a covering composite index. |
| 008-14 | Low | `ln -sfn` was described as atomic release activation but was not a guaranteed rename. | `scripts/deploy.sh`. | Fixed — same-directory temporary link plus `mv -Tf` used for activation and rollback; tested with Alpine BusyBox and Debian GNU coreutils. |
| 008-15 | Low / maintenance | `chrono`, `chrono-tz`, and `unsafe-libyaml` had unmaintained advisories, with no vulnerability failure reported. | `cargo audit` at review. | Reviewed — versions retained absent a compatible, justified replacement; final audit result recorded in Task 009. |
| 008-16 | Low | `axum-extra` appeared unused as a direct dependency. | `Cargo.toml` and source search. | Fixed — removed direct dependency; it remains transitively required by Loco. |

## Other review observations

- No cross-user ownership issue, SQL injection path, or user-controlled SSRF
  path was confirmed. Production API bodies are limited to 1 MiB; the CSP
  prevents script execution from inline script sources, and the service worker
  excludes private API routes.
- Password hashes use Loco's password hashing helpers. JWTs are stored in
  browser local storage, leaving the documented XSS exposure; current CSP and
  same-origin API restrictions reduce but do not eliminate that risk.
- Loco's SQLite connection setup applies WAL, `synchronous=NORMAL`, a 5-second
  busy timeout, and foreign keys on each connection. These defaults are kept.
- Search remains a bounded personal-inbox query rather than FTS; no measured
  evidence justified additional search infrastructure.
- Deployment packaging, credentials handling, online database backup, restore
  confirmation, and manual-only GitHub workflows were assessed as appropriate
  for the project's scale.
- Review test gaps included concurrent authentication/rate-limit behavior,
  transient preparation failures, backup ordering, binding drift, and mobile /
  accessibility assertions. Task 009 adds regressions for the first four;
  mobile/accessibility coverage remains deferred.

## Release recommendation at review time

Proceed after the medium findings were remediated and the full Docker-first CI
passed. No code was changed as part of Task 008.
