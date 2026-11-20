# Task 002 — Auth and Shell Hardening

**Status:** Complete (2026-11-18)

## Table of Contents

<!-- toc:start -->
- [Objective](#objective)
- [Scope](#scope)
- [Dependencies](#dependencies)
- [Implementation checklist](#implementation-checklist)
- [Implementation notes](#implementation-notes)
- [Acceptance criteria](#acceptance-criteria)
- [Validation commands](#validation-commands)
- [Validation results (2026-11-18)](#validation-results-2026-11-18)
- [Out of scope](#out-of-scope)
<!-- toc:end -->
## Objective

Make authentication production-shaped without replacing Loco's implementation:
configurable public registration (off by default), configurable email
verification, an administrator CLI task, and a hardened, installable frontend
shell.

## Scope

- Add `ALLOW_REGISTRATION` and `REQUIRE_EMAIL_VERIFICATION` to config, read
  through `ctx.config` (no `std::env::var` in application code).
- Gate `POST /api/auth/register` when registration is disabled.
- Enforce email verification on login when required.
- Add a Loco CLI task for creating the initial administrator and additional
  accounts, including `--verified`/`force` semantics.
- Hide the registration UI when registration is disabled (public config endpoint
  or a generated flag, kept outside the ten business endpoints).
- Harden the shell: auth state from `/api/auth/current`, loading/error states,
  logout, route guards, accessible navigation.
- Add a PWA manifest and a minimal service worker that never caches `/api/*` or
  credentials.
- Add Playwright smoke coverage for login → dashboard.
- Add tests for disabled registration and verification policy.

## Dependencies

- Task 001 complete.

## Implementation checklist

- [x] Add auth policy config to `config/{development,test,production}.yaml`.
- [x] Add a typed auth policy accessor on the app config (or a small model/config
      helper) used by the controller and tests.
- [x] Reject registration with a clear 403/404 response when disabled.
- [x] Enforce verification when configured, without breaking existing tests.
- [x] Add `task admin:create` (or extend `user:create`) with verified-user
      support and idempotent duplicate handling.
- [x] Expose the registration/verification flags to the SPA safely.
- [x] Render the register link/page only when registration is enabled.
- [x] Validate the JWT with `/api/auth/current` on protected routes.
- [x] Add PWA manifest, icons and service worker (app shell only).
- [x] Add Playwright config and a login/dashboard smoke test.
- [x] Update docs and PROGRESS.

## Implementation notes

- Policy lives in `settings.auth` and is read with `Config::settings<T>()` by
  `src/auth_policy.rs`. Application code never calls `std::env`.
- `POST /api/auth/register` returns `403` with the machine-readable code
  `registration_disabled` when disabled.
- `POST /api/auth/login` returns `403` with `email_verification_required` when
  the policy requires verification and the account is unverified.
- `GET /api/auth/config` is the only new public route. It returns the two
  booleans and no user data.
- `admin:create` creates a verified account by default, sends verification mail
  only when `verified:false`, and requires `force:true` to reset an existing
  account.
- JWT signing failures now return `500` instead of being disguised as a failed
  login. This surfaced a real deployment hazard: Loco base64-decodes
  `JWT_SECRET`, so a non-base64 secret breaks login.
- The SPA validates the stored token through `/api/auth/current`, shows a
  loading state while checking, clears the token and query cache on failure,
  hides registration when disabled, and provides an accessible skip link.
- The service worker caches only the app shell and static assets; `/api/*`,
  `/_*` and non-GET requests always go to the network.
- Docker-first CI was added in this milestone: `docker-compose.ci.yml`,
  `docker/ci/*.Dockerfile`, `scripts/ci.sh`, `scripts/ci-rust.sh`,
  `scripts/e2e-server.sh`, a production `Dockerfile` and `.dockerignore`.
  GitHub Actions is manual-only and calls `./scripts/ci.sh`.

## Acceptance criteria

- With `ALLOW_REGISTRATION=false`, `POST /api/auth/register` is rejected and the
  UI does not offer registration.
- With `REQUIRE_EMAIL_VERIFICATION=true`, an unverified user cannot log in and
  receives a clear error.
- The admin task creates a verified account non-interactively.
- Every protected API and page requires authentication.
- The PWA is installable and its service worker never caches `/api/*`.
- Backend and frontend checks pass, including the Playwright smoke test.

## Validation commands

Primary (Docker, one command):

```sh
./scripts/ci.sh
```

Equivalent host checks when Docker is unavailable:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --workspace

pnpm --dir frontend typecheck
pnpm --dir frontend lint
pnpm --dir frontend test
pnpm --dir frontend build
pnpm --dir frontend exec playwright test
```

## Validation results (2026-11-18)

`./scripts/ci.sh` passed end to end:

```text
Rust fmt / clippy / tests          passed — 39 tests
SQLite migrations                  passed — clean throwaway database
Frontend typecheck / lint / tests  passed — 3 tests
Frontend production build          passed
Production image + /_health        passed
Playwright smoke tests             passed — 6 tests
```

## Out of scope

- Items, settings and stats (milestone 3).
- AI and notifications (milestones 4–5).
- Deployment compose, image publishing and backups (milestone 6).
- Replacing Loco auth with sessions, OAuth or a separate user system.
- Multi-factor authentication.
