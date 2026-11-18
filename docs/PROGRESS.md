# BrainDrop Progress

Last updated: **2026-11-18**

## Milestones

| # | Milestone | Status | Notes |
|---|---|---|---|
| 001 | Bootstrap | **Complete** | Docs, scaffold verification, SQLite + SQLite queue, Mantine shell, builds and tests |
| 002 | Auth and shell hardening | Not started | Registration/verification config, admin task, PWA, Playwright smoke |
| 003 | Items and settings | Not started | Items CRUD, filtering, pagination, stats, settings, item UI |
| 004 | AI capture | Not started | OpenRouter client, `/api/ai/parse`, review UI |
| 005 | Notifications | Not started | Telegram/Discord, delivery queue, scheduler scans |
| 006 | Deployment | Not started | Docker, compose, CI/GHCR, TierHive template, backups |
| 007 | Release | Not started | Smoke tests, security review, docs, release |

Work proceeds one milestone at a time. Do not start milestone 002 until milestone
001 is reviewed.

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
  the Codex co-author identity.

### Blockers

None.

### Known limitations

- The quick-capture modal is a shell placeholder; it does not persist items yet.
- Dashboard counts are placeholders (`—`) until `/api/stats` exists.
- Settings and Integrations pages are informational placeholders.
- Registration is still the generated open flow; milestone 002 adds
  `ALLOW_REGISTRATION` and the admin task.
- No Docker, CI publishing, backup scripts or PWA yet.
- `.github/workflows/ci.yaml` is the generated workflow and still references
  Redis/Postgres and npm. It is replaced in milestone 006; do not treat it as
  the current CI contract.
- Development SMTP requires a local catcher (e.g. Mailpit) for verification mail.

## Next task

**002 — Auth and shell hardening.** See `docs/tasks/002-auth-and-shell.md`.
