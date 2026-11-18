# Task 002 — Auth and Shell Hardening

**Status:** Not started

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

- [ ] Add auth policy config to `config/{development,test,production}.yaml`.
- [ ] Add a typed auth policy accessor on the app config (or a small model/config
      helper) used by the controller and tests.
- [ ] Reject registration with a clear 403/404 response when disabled.
- [ ] Enforce verification when configured, without breaking existing tests.
- [ ] Add `task admin:create` (or extend `user:create`) with verified-user
      support and idempotent duplicate handling.
- [ ] Expose the registration/verification flags to the SPA safely.
- [ ] Render the register link/page only when registration is enabled.
- [ ] Validate the JWT with `/api/auth/current` on protected routes.
- [ ] Add PWA manifest, icons and service worker (app shell only).
- [ ] Add Playwright config and a login/dashboard smoke test.
- [ ] Update docs and PROGRESS.

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

## Out of scope

- Items, settings and stats (milestone 3).
- AI and notifications (milestones 4–5).
- Docker/CI publishing (milestone 6).
- Replacing Loco auth with sessions, OAuth or a separate user system.
- Multi-factor authentication.
