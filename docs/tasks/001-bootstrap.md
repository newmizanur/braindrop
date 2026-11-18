# Task 001 — Bootstrap

**Status:** Complete (2026-11-18)

## Objective

Establish BrainDrop on top of the official Loco scaffold: preserve generated
authentication and structure, verify SQLite + the SQLite-backed queue, add project
documentation, extend the React SPA with Mantine, and deliver a minimal,
buildable application shell.

## Scope

- Inspect the repository and installed tooling before changing anything.
- Preserve the existing Loco scaffold (generated with SQLite, `queue-sqlite`,
  `clientside` assets).
- Verify SQLite configuration and WAL behavior.
- Verify the generated authentication and its tests.
- Create `README.md`, `LICENSE`, and the `docs/` set including all seven task
  files.
- Add Mantine and supporting frontend tooling (hooks, dates, notifications,
  Tabler Icons, dayjs, PostCSS preset).
- Build the application shell: `AppShell`, responsive navigation, theme toggle,
  auth screens, placeholder product pages.
- Add ESLint, Vitest and a type-check script.
- Run backend and frontend validation.
- Initialize Git (the directory was not a repository) and commit the milestone.

## Dependencies

None. This is the first milestone.

## Implementation checklist

- [x] Inspect repository contents and Git status.
- [x] Confirm installed tools (`loco` 1.3.1, `sea-orm-cli` 2.0.4, Rust stable,
      Node, pnpm).
- [x] Review official Loco docs for CLI, app generation and static/SPA assets.
- [x] Preserve the generated Loco app (no re-scaffold, no nested repo).
- [x] Confirm `database.uri` uses SQLite with `mode=rwc`.
- [x] Confirm `queue.kind: Sqlite` and a separate queue database.
- [x] Confirm Loco enables WAL for file-backed SQLite.
- [x] Verify generated auth routes and tests are intact.
- [x] Write `README.md` and `LICENSE`.
- [x] Write `docs/SPEC.md`, `docs/ARCHITECTURE.md`, `docs/PROGRESS.md`,
      `docs/DEPLOYMENT.md`.
- [x] Write tasks 001–007.
- [x] Install Mantine packages and Mantine PostCSS preset.
- [x] Add `MantineProvider`, notifications, theme and global styles.
- [x] Build the `AppShell` layout with mobile navigation and user menu.
- [x] Restyle login and registration with Mantine.
- [x] Add placeholder routes: Dashboard, Inbox, Today, Upcoming, All items,
      Completed, Settings, Integrations.
- [x] Add a quick-capture modal placeholder.
- [x] Add TanStack Query current-user hook.
- [x] Add a JSON 404 catch-all for unknown `/api/*` routes and a request test.
- [x] Add ESLint, Vitest, type-check script and a basic test.
- [x] Add `.gitignore` coverage for env files and build output.
- [x] Initialize Git and commit.

## Acceptance criteria

- The app boots with the generated config and serves the built SPA from
  `frontend/dist`; direct refreshes of client-side routes return the SPA.
- Unknown `/api/*` routes return a JSON 404, not the SPA.
- `cargo loco routes` shows the generated auth routes and monitoring endpoints.
- Generated authentication tests pass unchanged.
- The frontend type-checks, lints, tests and builds.
- The shell is usable on mobile and desktop, supports light/dark mode, and every
  placeholder page is reachable.
- No secrets, database files, `node_modules`, `target/` or build artifacts are
  committed.
- Documentation describes the current state and the remaining milestones.

## Validation commands

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --workspace

pnpm --dir frontend typecheck
pnpm --dir frontend lint
pnpm --dir frontend test
pnpm --dir frontend build

cargo loco routes
cargo loco doctor
```

## Out of scope

- Items, settings, stats and all ten business endpoints (milestone 3).
- Registration toggle, verification policy, admin task and PWA (milestone 2).
- OpenRouter/AI parsing (milestone 4).
- Telegram/Discord notifications, delivery queue and scheduler scans (milestone 5).
- Docker, Compose, CI publishing, backups and TierHive deployment (milestone 6).
- Playwright smoke tests and release hardening (milestones 2 and 7).
- Embedded binary assets.
