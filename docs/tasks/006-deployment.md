# Task 006 — Deployment

**Status:** Not started

## Objective

Make BrainDrop deployable as a self-hosted single-node application: reproducible
container images, persistent SQLite storage, CI checks, image publishing,
backup/restore and a verified TierHive-compatible deployment template.

## Scope

- Multi-stage `Dockerfile` (Node frontend build, Rust build, minimal runtime).
- `docker-compose.yml`, `.dockerignore`, `.env.example`.
- Non-root runtime user; persistent volume for SQLite + queue + WAL files.
- GitHub Actions CI (Rust + frontend + Docker build).
- GHCR image publishing.
- TierHive-compatible SSH deployment template (verify platform capabilities first).
- SQLite online backup and restore scripts.
- Production deployment documentation.
- Verify persistence across container restarts.

## Dependencies

- Tasks 001–005 substantially complete (at minimum 001 and 003).
- Verified TierHive platform capabilities and authorized deployment access.

## Implementation checklist

- [ ] Write the multi-stage Dockerfile with pinned toolchain versions.
- [ ] Build the SPA in the Node stage and copy `frontend/dist` into the runtime.
- [ ] Build the release binary; strip or minimize the runtime image.
- [ ] Run as a non-root user; set correct volume permissions.
- [ ] Add Compose with server, worker, scheduler (or `--all`) and a volume.
- [ ] Add `.dockerignore` (target, node_modules, DBs, `.env`, docs build output).
- [ ] Add `.env.example` with placeholders only.
- [ ] Add CI: fmt, clippy, test, frontend typecheck/lint/test/build, Docker build.
- [ ] Add GHCR publish workflow with immutable tags and `GITHUB_TOKEN`.
- [ ] Add backup/restore scripts using SQLite `.backup` and integrity checks.
- [ ] Add health checks for `/_health` and `/_readiness`.
- [ ] Verify clean Docker build and restart persistence.
- [ ] Add TierHive SSH template with secrets supplied out-of-band.
- [ ] Finish `docs/DEPLOYMENT.md` with real, verified commands.
- [ ] Update PROGRESS.

## Acceptance criteria

- `docker build` succeeds from a clean checkout with no external credentials.
- The container serves the SPA and API, runs migrations, and passes health checks.
- Data survives container removal and recreation when the volume is retained.
- Runtime image contains no compiler, Node.js or pnpm and runs as non-root.
- CI is green and publishes an image to GHCR on the configured trigger.
- Backup/restore is documented and tested against a real SQLite database.
- No production deployment occurs without explicit authorization.

## Validation commands

```sh
docker build -t braindrop:test .
docker compose config
docker compose up -d
curl -fsS localhost:5150/_health
./scripts/backup.sh
./scripts/restore.sh <backup>
docker compose down
docker compose up -d
```

Plus the full backend/frontend check suite.

## Out of scope

- Kubernetes manifests or autoscaling.
- Multi-replica SQLite clustering.
- Managed database migration.
- Domain/DNS/TLS provisioning inside the app.
- Publishing a production release without authorization.
