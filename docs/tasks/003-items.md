# Task 003 — Items and Settings

**Status:** Complete (2026-11-18)

## Table of Contents

<!-- toc:start -->
- [Objective](#objective)
- [Scope](#scope)
- [Dependencies](#dependencies)
- [Implementation checklist](#implementation-checklist)
- [Acceptance criteria](#acceptance-criteria)
- [Validation commands](#validation-commands)
- [Out of scope](#out-of-scope)
<!-- toc:end -->
## Objective

Implement the core product: items (tasks, notes, ideas, reminders), user settings,
dashboard statistics, and the matching SPA pages. Manual capture must work
without AI.

## Scope

- Migrations and SeaORM models for `items` and `user_settings`.
- Ten-endpoint API minus AI and notification test:
  - `GET/POST /api/items`
  - `GET/PATCH/DELETE /api/items/:id`
  - `GET /api/stats`
  - `GET/PATCH /api/settings`
- Typed DTOs, validation, ts-rs bindings, consistent errors.
- Strict per-user ownership on every query.
- Filtering, sorting, pagination and text search.
- Frontend: real Dashboard, Inbox, Today, Upcoming, All items, Completed,
  Settings pages; quick capture; edit/delete; filters; pagination.
- Dashboard statistics from `/api/stats`.
- Tests: CRUD, validation, pagination, filters, cross-user isolation, stats.

## Dependencies

- Task 001 complete.
- Task 002 recommended for auth policy, but items can be built on the generated
  auth if 002 is still pending.

## Implementation checklist

- [x] Generate the `items` model/migration with `cargo loco generate`; add
      indexes and foreign key.
- [x] Generate the `user_settings` model/migration; unique `user_id`.
- [x] Implement model finders and state transitions (create, update, complete,
      archive, scoped find).
- [x] Define request/response DTOs and export TypeScript bindings.
- [x] Implement the items controller with JWT auth and ownership checks.
- [x] Implement stats aggregation scoped to the user.
- [x] Implement settings get/update; never return secrets.
- [x] Add request tests for auth, ownership, validation, filters and pagination.
- [x] Add model tests and snapshots where appropriate.
- [x] Build the item list, item editor, quick capture and filters in the SPA.
- [x] Wire the dashboard cards and Today/Upcoming/Completed views.
- [x] Add loading, empty and error states.
- [x] Keep unknown `/api/*` routes returning JSON 404 (implemented in 001).
- [x] Update docs and PROGRESS.

## Acceptance criteria

- A signed-in user can create one or many items, list/filter/sort/paginate,
  retrieve, update and delete only their own items.
- Another user's item id returns 404 for GET/PATCH/DELETE.
- Manual capture works with no AI configuration.
- Settings persist per user and never expose credentials.
- Dashboard statistics match the user's data.
- All pages are responsive and usable with keyboard only.
- Backend and frontend checks pass.

## Validation commands

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --workspace

pnpm --dir frontend typecheck
pnpm --dir frontend lint
pnpm --dir frontend test
pnpm --dir frontend build
```

## Out of scope

- AI parsing and review (milestone 4).
- Notification delivery and preferences UI beyond storage (milestone 5).
- Bulk import/export.
- Sharing, teams or collaboration.
- FTS5 if a simpler indexed search meets the acceptance criteria.
