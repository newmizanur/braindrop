# Task 007 — Release

**Status:** Not started

## Objective

Harden and document BrainDrop for a first open-source release: complete test
coverage, security review, release documentation, versioning and a repeatable
release checklist.

## Scope

- Playwright end-to-end smoke tests.
- Security review: authz, secrets, SSRF, XSS/CSP, service worker caching.
- Full backend/frontend validation.
- Documentation review and a changelog.
- Version bump to `0.1.0` and release notes.
- License and repository hygiene check.
- Release checklist; no unauthorized push/deploy.

## Dependencies

- Tasks 001–006 complete.

## Implementation checklist

- [ ] Add Playwright smoke tests for login, dashboard, quick capture and a
      representative item flow.
- [ ] Add cross-user isolation tests for every business endpoint.
- [ ] Review JWT storage/XSS posture and add a CSP if appropriate.
- [ ] Review service worker scope: no `/api/*` or credential caching.
- [ ] Verify AI and notification secrets never appear in responses or logs.
- [ ] Verify SSRF restrictions on notification destinations.
- [ ] Run the full backend/frontend/Docker suites from a clean checkout.
- [ ] Write `CHANGELOG.md` and release notes.
- [ ] Confirm `LICENSE`, `README`, docs and `.env.example` are accurate.
- [ ] Confirm no secrets, DB files, `target/`, `node_modules` or build artifacts
      are tracked.
- [ ] Bump the version and tag only with authorization.
- [ ] Update PROGRESS and close the milestone.

## Acceptance criteria

- All automated checks pass from a clean checkout with no external credentials.
- The end-to-end smoke suite covers the primary user journey.
- Authorization and cross-user isolation are tested, not just assumed.
- No high-severity security issues remain in the review.
- Documentation matches the released behavior.
- Release artifacts are reproducible and contain no secrets.
- Nothing is pushed, merged or deployed without explicit authorization.

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

docker build -t braindrop:release .
```

## Out of scope

- Post-1.0 features.
- Multi-tenant teams, sharing or collaboration.
- Horizontal scaling.
- Hosted SaaS operations.
- Publishing a release without explicit authorization.
