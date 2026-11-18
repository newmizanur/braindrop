# BrainDrop Progress

Last updated: **2026-11-18**

## Milestones

| # | Milestone | Status | Notes |
|---|---|---|---|
| 001 | Bootstrap | **Complete** | Docs, scaffold verification, SQLite + SQLite queue, Mantine shell, builds and tests |
| 002 | Auth and shell hardening | **Complete** | Registration/verification config, admin task, PWA, Playwright smoke, Docker-first CI |
| 003 | Items and settings | Not started | Items CRUD, filtering, pagination, stats, settings, item UI |
| 004 | AI capture | Not started | OpenRouter client, `/api/ai/parse`, review UI |
| 005 | Notifications | Not started | Telegram/Discord, delivery queue, scheduler scans |
| 006 | Deployment | Not started | Deployment compose, GHCR publishing, TierHive template, backups (image and CI already delivered) |
| 007 | Release | Not started | Smoke tests, security review, docs, release |

Work proceeds one milestone at a time. Do not start milestone 003 until milestone
002 is reviewed.

## Milestone 001 — Bootstrap

### Completed

- Inspected the existing repository before changing anything. A complete Loco
  scaffold was already present (SQLite, `queue: kind: Sqlite`, `--assets
  clientside`), so it was preserved instead of regenerated.
- Confirmed the toolchain:
  - `loco` 1.3.1, `loco-rs` 1.3.1
  - `sea-orm-cli` 2.0.4
  - Rust 1.100.0, Cargo 1.100.0
  - Node 24.14.1, pnpm 10.12.4 (via Corepack), npm 11.11.0
- Confirmed SQLite configuration in `config/{development,test,production}.yaml`:
  - app DB: `sqlite://braindrop_<env>.sqlite?mode=rwc`
  - queue: `kind: Sqlite`, `sqlite://loco_<env>.sqlite?mode=rwc`
  - Loco 1.3 enables WAL automatically for file-backed SQLite.
- Preserved generated authentication (`/api/auth/*`), users model, mailers,
  tasks, workers and tests.
- Added project documentation: `README.md`, `LICENSE`, `docs/SPEC.md`,
  `docs/ARCHITECTURE.md`, `docs/PROGRESS.md`, `docs/DEPLOYMENT.md`, and the
  seven milestone task files.
- Extended the React SPA with Mantine 9:
  - `MantineProvider`, Mantine core/dates/notifications styles
  - AppShell with responsive navbar, mobile burger, header, user menu
  - light/dark/system theme toggle
  - quick-capture modal placeholder
  - login and registration screens restyled
  - placeholder pages for Dashboard, Inbox, Today, Upcoming, All items,
    Completed, Settings and Integrations
  - TanStack Query current-user hook
  - shared empty/loading/error building blocks
- Added an explicit `/api` catch-all controller so unknown API routes return a
  JSON 404 instead of the SPA `index.html`.
- Added frontend tooling: ESLint, Vitest, type-check script, PostCSS Mantine
  preset, Vitest setup.
- Initialized a Git repository (the directory was not a repo) and committed the
  milestone as `chore: bootstrap BrainDrop on Loco with a Mantine shell`.

### Validation

Backend:

```text
cargo fmt --all -- --check        passed
cargo clippy --all-targets -- -D warnings
                                  passed
cargo test                        passed — 29 tests (2 unit, 27 request/model/task)
cargo loco routes                 auth + monitoring + /api fallback registered
```

Frontend:

```text
pnpm --dir frontend typecheck     passed
pnpm --dir frontend lint          passed
pnpm --dir frontend test          passed — 3 tests
pnpm --dir frontend build         passed — dist/index.html + hashed JS/CSS
```

Runtime smoke test (`PORT=5199 cargo loco start`):

```text
GET /_health              200
GET /                     200 text/html
GET /dashboard            200 text/html (SPA fallback)
GET /api                  404 application/json
GET /api/does-not-exist   404 application/json
GET /api/auth/does-not-exist 404 application/json
```

The frontend build reports a >500 kB chunk warning (Mantine + Tabler icons).
It is not a failure; route-level code splitting is deferred.

### Decisions

- **Keep Loco 1.3.1.** The delivered scaffold is complete and current. The
  specification names 1.2.x, but downgrading would discard a working generated
  app (SeaORM 2, React 19, React Router 8) and risks breaking generated auth and
  tests. The conventions are unchanged; the deviation is documented in
  `ARCHITECTURE.md`.
- **Do not regenerate.** `loco new` output is already present; re-running it
  would overwrite the repository. The generator compatibility was verified by
  inspecting the generated structure and installed tool versions.
- **Minimal shell only.** Item persistence, AI and notifications are deliberately
  absent so milestone 001 stays independently testable.
- **Commit without co-author** for milestone 001, per the initial execution
  instructions. Later implementation commits follow the repository rule and add
  the implementing agent (Codex or OpenCode) as co-author.

### Blockers

None.

### Known limitations

- The quick-capture modal is a shell placeholder; it does not persist items yet.
- Dashboard counts are placeholders (`—`) until `/api/stats` exists.
- Settings and Integrations pages are informational placeholders.
- Registration was still the generated open flow and there was no Docker, CI
  publishing, backup script or PWA yet; milestone 002 replaced registration with
  the configurable policy and added Docker-first CI and the PWA shell.
- `.github/workflows/ci.yaml` was the generated workflow and referenced
  Redis/Postgres and npm; milestone 002 replaced it with a manual-only workflow
  that calls `./scripts/ci.sh`.
- Development SMTP requires a local catcher (e.g. Mailpit) for verification mail.

## Milestone 002 — Auth and Shell Hardening

### Completed

- Added a typed authentication policy in `src/auth_policy.rs`
  (`AuthPolicy::from_config`), read from `settings.auth` through
  `Config::settings<T>()`. Application code never calls `std::env`.
  - `ALLOW_REGISTRATION` (default `false` in development/production).
  - `REQUIRE_EMAIL_VERIFICATION` (default `false`).
- Gated `POST /api/auth/register`: returns `403` with the machine-readable code
  `registration_disabled` when public registration is off.
- Enforced email verification on login when configured: returns `403`
  `email_verification_required` for unverified accounts.
- Added the public `GET /api/auth/config` route, which returns only the two
  policy booleans. It sits outside the ten business endpoints and needs no token.
- Added the native Loco task `admin:create` (`src/tasks/admin_create.rs`):
  - creates a verified account by default;
  - `verified:false` creates an unverified account and sends the verification
    mail;
  - `force:true` resets an existing account;
  - duplicate accounts fail without `force`, so repeated setup is explicit.
- Fixed a real deployment hazard: Loco base64-decodes `JWT_SECRET` before
  signing, and a signing failure was previously reported as `401`. Token
  signing now lives in `issue_token` and returns `500` with a clear log message,
  and `config/production.yaml` documents that the secret must be base64.
- Hardened the SPA shell:
  - `RequireAuth` validates the stored token through `/api/auth/current`, shows
    `FullPageLoader` while checking, and clears token + query cache on failure.
  - Login and Register read the public auth config; the register link and route
    are hidden when registration is disabled.
  - `api/client.ts` parses both the Loco error body and the shared `ApiError`
    shape; logout clears client state.
  - Accessible skip link, focus styles and shell copy updates.
- Added PWA installability:
  - `frontend/public/manifest.webmanifest`, `favicon.svg` and generated PNG
    icons.
  - `frontend/public/sw.js` caches only the app shell and static assets;
    `/api/*`, `/_*` and all non-GET requests always go to the network.
  - Registered in production builds only (`frontend/src/pwa.ts`), never in
    development.
- Added Playwright smoke coverage (`frontend/playwright.config.ts`,
  `frontend/e2e/auth.spec.ts`, `frontend/e2e/pwa.spec.ts`): protected-route
  redirect, registration hidden when disabled, login → dashboard → logout,
  bad-credential handling without a full reload, manifest delivery and
  service-worker registration (asserting no `/api/*` path is cached).
- Added Docker-first local CI:
  - production multi-stage `Dockerfile` (pnpm frontend build, Rust release
    build, slim non-root runtime, health check) and `.dockerignore`;
  - `docker/ci/{rust,frontend,playwright}.Dockerfile` and
    `docker-compose.ci.yml`;
  - `scripts/ci.sh` as the single entry point, plus `scripts/ci-rust.sh` and
    `scripts/e2e-server.sh`;
  - Cargo caches in named Docker volumes and the pnpm store in a BuildKit cache
    mount; test databases live inside
    containers, never in the repository;
  - Playwright runs in the app container's network namespace over
    `127.0.0.1:5150`, because Chromium upgrades non-localhost HTTP origins to
    HTTPS.
- Replaced `.github/workflows/ci.yaml` with a manual-only
  (`workflow_dispatch`) workflow that calls `./scripts/ci.sh`, so hosted and
  local runs cannot drift. Automatic runs are intentionally disabled for now.
- Updated `AGENTS.md`, `README.md`, `docs/SPEC.md`, `docs/ARCHITECTURE.md`,
  `docs/DEPLOYMENT.md` and `docs/tasks/002-auth-and-shell.md` with the new auth
  policy and CI contract.

### Validation

Single command, Docker-first:

```text
./scripts/ci.sh                   passed end to end
```

Contents:

```text
Rust fmt / clippy / tests         passed — 39 tests (4 unit, 35 integration)
SQLite migrations                 passed — throwaway database
Frontend typecheck / lint / tests passed — 3 Vitest tests
Frontend production build         passed
Production image + /_health       passed
Playwright smoke tests            passed — 6 tests
```

### Decisions

- **Policy on `settings.auth`.** Loco already deserializes typed settings;
  a small typed struct is less error-prone than reading keys ad hoc and keeps
  the flags testable through `ctx.config`.
- **Registration disabled by default.** The specification requires it; tests
  enable registration explicitly so the generated auth flow stays covered.
- **New `admin:create` task instead of changing `user:create`.** The generated
  task keeps its meaning; the new task carries verification and `force`
  semantics and is the documented way to create the first account.
- **Playwright shares the app's network namespace.** It exercises the real
  production image on `127.0.0.1`, which also avoids Chromium's HTTPS upgrade
  for non-localhost origins.
- **CI builds the production image now.** Milestone 6 still owns full deployment
  Compose, publishing, backups and the TierHive template, but validating the
  image early prevents an unbuildable deployment from accumulating.

### Blockers

None.

### Known limitations

- Quick capture remains a shell placeholder; it does not persist items until
  milestone 3.
- Dashboard counts and Settings/Integrations pages are still placeholders.
- `docker-compose.ci.yml` is a CI harness, not the deployment compose file.
  Publishing, backups and the TierHive template remain milestone 6 work.
- GitHub Actions is manual-only by instruction; it has not been run on GitHub in
  this milestone.
- The frontend build still reports a >500 kB chunk warning (Mantine + Tabler
  icons); route-level code splitting is deferred.
- No strict Content-Security-Policy yet. The JWT stays in `localStorage`, as
  generated by Loco; CSP/XSS hardening remains in milestone 7.
- Development SMTP requires a local catcher (e.g. Mailpit) for verification and
  reset mail.

## Next task

**003 — Items and settings.** See `docs/tasks/003-items.md`.
