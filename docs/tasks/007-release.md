# Task 007 — Release

**Status:** Complete — `READY WITH LIMITATIONS`. Nothing was pushed, tagged,
published or deployed; those steps require explicit authorization.

## Objective

Harden and document BrainDrop for a first open-source release: complete test
coverage, security review, release documentation, versioning and a repeatable
release checklist.

## Scope

- Playwright end-to-end smoke tests.
- Security review: authz, secrets, SSRF, XSS/CSP, service worker caching.
- Full backend/frontend validation.
- Documentation review and a changelog.
- Versioning and release notes (`0.1.0` already set at HEAD; verified rather
  than re-bumped).
- License and repository hygiene check.
- Release checklist; no unauthorized push/deploy.

## Dependencies

- Tasks 001–006 complete.

## Implementation checklist

- [x] Add Playwright smoke tests for login, dashboard, quick capture and a
      representative item flow.
- [x] Add cross-user isolation tests for every business endpoint (covered by
      the Tasks 003–005 suites; retained in the final run).
- [x] Review JWT storage/XSS posture and add a strict CSP (kept the generated
      `localStorage` JWT; documented residual risk and compensating controls).
- [x] Review service worker scope: no `/api/*` or credential caching.
- [x] Verify AI and notification secrets never appear in responses or logs.
- [x] Verify SSRF restrictions on notification destinations (official hosts
      only, no redirects; OpenRouter redirects disabled).
- [x] Run the full backend/frontend/Docker suites from a clean checkout.
- [x] Write `CHANGELOG.md` and release notes.
- [x] Confirm `LICENSE`, `README`, docs and `.env.example` are accurate.
- [x] Confirm no secrets, DB files, `target/`, `node_modules` or build artifacts
      are tracked.
- [x] Verify version consistency (no bump was needed); tag only with
      authorization.
- [x] Update PROGRESS and close the milestone.

## Acceptance criteria

- All automated checks pass from a clean checkout with no external credentials.
- The end-to-end smoke suite covers the primary user journey.
- Authorization and cross-user isolation are tested, not just assumed.
- No high-severity security issues remain in the review.
- Documentation matches the released behavior.
- Release artifacts are reproducible and contain no secrets.
- Nothing is pushed, merged or deployed without explicit authorization.

All criteria are met. The classification is `READY WITH LIMITATIONS`; the
limitations are documented in `docs/PROGRESS.md` and
`docs/releases/v0.1.0.md`.

## Validation commands

```sh
./scripts/ci.sh

# Additional release checks (host-side)
cargo audit
pnpm --dir frontend audit --audit-level=moderate
cargo loco doctor                  # LOCO_ENV=production, placeholder secrets
./scripts/deploy.sh validate
./scripts/package.sh --check dist/braindrop-linux-amd64-musl
```

## Decisions

- JWT stays in `localStorage` for v0.1.0; no partial cookie migration.
- Explicit `secure_headers` with `preset: empty`; HSTS only at a TLS proxy.
- Authentication rate limits are in-process, single-replica only, and rely on
  correctly configured `REMOTE_IP_SOURCE`.
- Login consumes only failures and clears on success; duplicate registration
  uses a neutral response to avoid account enumeration.
- Local archives are verification artifacts; the manual release workflow
  rebuilds all six from the tagged commit.

## Out of scope

- Post-1.0 features.
- Multi-tenant teams, sharing or collaboration.
- Horizontal scaling.
- Hosted SaaS operations.
- Publishing a release without explicit authorization.
