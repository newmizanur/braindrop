# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-19

First release of BrainDrop. This entry collects everything delivered in
milestones 001–006 plus the Task 007 release hardening. Version `0.1.0` is set
in `Cargo.toml` and `frontend/package.json`.

### Added

- **Bootstrap and shell (001):** Loco scaffold with SQLite and the SQLite-backed
  queue, a client-side React SPA, Mantine `AppShell` with light/dark/system
  themes, responsive navigation, authentication screens and a JSON 404
  catch-all for unknown `/api/*` routes.
- **Authentication and CI (002):** configurable registration and email
  verification, public `/api/auth/config`, the `admin:create` task for the first
  account, PWA installability with a service worker that never caches API
  responses, Playwright smoke tests, and the Docker-first `./scripts/ci.sh`
  pipeline with a manual-only GitHub Actions workflow that calls the same
  script.
- **Items and settings (003):** `items` and `user_settings` tables, typed DTOs
  with generated TypeScript bindings, owner-scoped CRUD (single or batch with
  transactional insert), input search (indexed `LIKE`), filtering, sorting,
  pagination, timezone-aware dashboard statistics, item views and Settings.
- **Optional AI capture (004):** a minimal OpenRouter client behind an
  `AiProvider` trait, `POST /api/ai/parse`, strict validation of reviewable
  suggestions, timezone-aware relative dates, encrypted per-user (BYOK) keys,
  Integrations controls and a quick-capture review flow. Manual capture always
  remains available and needs no provider.
- **Notifications (005):** independent Telegram and Discord channels,
  `notification_deliveries` with durable deduplication keys, a delivery worker
  with database-owned retries/backoff, five scheduled scans (reminders, digests,
  overdue, recovery, cleanup), notification settings and Integrations UI, and
  `POST /api/notifications/test`.
- **Deployment and distribution (006):** production Docker Compose, six native
  release targets, `Makefile` and an `ssh`/`scp` release flow with atomic
  activation and rollback, online SQLite backup/restore scripts, distribution
  archives with SHA-256 sidecars, and deployment checks in CI.
- **Release documentation:** `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`,
  `SECURITY.md`, this changelog and `docs/releases/v0.1.0.md`, plus README
  documentation of security posture and screenshots.

### Changed

- The SPA now lazy-loads quick capture and splits vendor bundles in Vite to keep
  the initial bundle focused on the shell and authentication.
- Vite/HTML changes move the pre-paint colour-scheme logic into an external
  script so the production CSP needs no inline scripts.
- Accessibility refinements: higher-contrast `dimmed` tokens, explicit button
  types in the confirmation dialog and clearer sort-direction labels.
- Auth responses avoid account enumeration: duplicate registration uses the
  same neutral success response as a new registration, unknown-email logins
  perform a dummy password hash, and log messages no longer include email
  addresses. Verification tokens expire after 24 hours.
- Magic-link sign-in honours the email-verification policy and consumes the
  one-time link even when verification is still required.
- GitHub Actions uses commit-pinned third-party actions; the workflows remain
  manual-only and still call the same scripts as local CI.

### Security

- Added explicit security headers in every environment: a strict
  `Content-Security-Policy`, `X-Content-Type-Options: nosniff`,
  `X-Frame-Options: DENY`, `Referrer-Policy: no-referrer`, `Permissions-Policy`,
  `Cross-Origin-Opener-Policy: same-origin` and
  `Cross-Origin-Resource-Policy: same-origin`. HSTS remains the reverse proxy's
  responsibility because the app does not see TLS.
- Added a cache-control middleware that sends `Cache-Control: no-store` for
  `/api/*`, monitoring paths and served HTML; hashed static assets stay
  cacheable. The service worker already bypasses API and monitoring paths.
- Added in-process, cache-backed rate limits for authentication endpoints,
  keyed by account/token and client address, with the trusted proxy source
  controlled by `REMOTE_IP_SOURCE` (default `ConnectInfo`).
- Added a 1 MiB request-body limit and validation for registration, password
  reset and reset-token fields (email format and minimum lengths).
- Disabled redirects in the OpenRouter HTTP client so a key is not leaked to a
  redirect target.
- Redacted database error details from notification API errors and delivery
  records; details stay in server logs.
- Hardened SSH deployment staging: uploads use an unpredictable `mktemp` path
  under the SSH user's home with `umask 077`, staging directories are `0700`,
  and a cleanup trap removes both archive and staging on success or failure.

### Known limitations

- External notification delivery is at-least-once, not exactly-once.
- Authentication and notification test rate-limit counters are in memory and
  reset on restart; edge limits are recommended for public exposure.
- The frontend still stores the JWT in `localStorage`; the strict CSP and
  no-store policy reduce but do not remove that residual XSS risk.
- SQLite is single-writer; one application replica is supported.
- An actual SSH deployment, real Alpine/Ubuntu service boot and TierHive
  behaviour were not exercised automatically and are documented, not claimed.
