# Task 006 — Deployment and distribution

**Status:** Implemented. Full Docker CI passes. Native artefacts are built and
measured; an actual SSH deployment and a real Alpine/Ubuntu boot remain
authorized-operator steps and are documented as unverified.

## Objective

Make BrainDrop deployable and distributable as a self-hosted single-node
application without requiring Docker on the target: production container images,
native binaries for six targets, a repeatable SSH release flow, consistent
backups and honest deployment documentation.

## Scope

### 006A — Docker and Compose

- [x] Multi-stage `Dockerfile`: pnpm SPA build, Rust release build, slim
      non-root runtime.
- [x] Release profile tuned for shipping (`lto = "thin"`, `codegen-units = 1`,
      `strip = "symbols"`, `panic = "unwind"`).
- [x] `docker-compose.yml` for production with named volume, `env_file`, health
      check, restart policy, `init`, stop grace period.
- [x] Keep `docker-compose.ci.yml` separate and test-only.
- [x] `.env.example` with placeholders only.
- [x] Mail disabled by default so a manual-only install boots without SMTP.
- [x] Image runs `start --all` and contains `sqlite3`, `tar`, `gzip`,
      `findutils` for in-container backups.
- [x] CI persistence test: create an item, restart the container, verify it.

### 006B — Native builds and distribution

- [x] Six targets: musl AMD64/ARM64, glibc AMD64/ARM64, macOS AMD64/ARM64.
- [x] `scripts/build-native.sh` — musl via `messense/rust-musl-cross`, glibc via
      `rust:1-bullseye` (glibc 2.31 baseline), macOS natively.
- [x] `scripts/package.sh` — `dist/braindrop-<platform>/` plus `.tar.gz` and
      `.sha256`, with layout checks.
- [x] `scripts/build-frontend.sh` using the pinned pnpm version.
- [x] Archives exclude databases, real `.env`, `node_modules`, source and build
      output.

### 006C — Makefile and SSH deployment

- [x] Documented `Makefile` with the required variables and targets.
- [x] `scripts/deploy.sh` using only `ssh`/`scp`; no `rsync`, Docker or compiler
      on the server.
- [x] Argument validation, staged uploads, atomic `current` symlink, preserved
      `shared/.env` and data, health check, release retention, rollback without
      database downgrade.
- [x] `scripts/deploy-init-alpine.sh` (OpenRC + `supervise-daemon`).
- [x] `scripts/deploy-init-ubuntu.sh` (systemd, unprivileged user, hardening).
- [x] Host-key verification left on; local `.env` never uploaded.

### 006D — Backup and restore

- [x] `scripts/backup.sh` using SQLite `.backup`, integrity check, timestamped
      archive, checksum, retention.
- [x] `scripts/restore.sh` with confirmation, checksum and integrity checks,
      previous databases moved aside, optional migrations and health check.
- [x] `scripts/test-backup-restore.sh` exercising real rows, settings and a
      pending queue job; runs in CI.

### 006E — Validation, measurements, documentation

- [x] Deployment checks wired into `./scripts/ci.sh`.
- [x] Resource measurements recorded below.
- [x] `README.md`, `AGENTS.md`, `docs/SPEC.md`, `docs/ARCHITECTURE.md`,
      `docs/DEPLOYMENT.md`, `docs/PROGRESS.md` updated.
- [x] GitHub Actions stays manual-only; no publishing workflow is triggered and
      no image, archive or release is published.

## Acceptance criteria

- [x] `docker build` succeeds from a clean checkout with no external credentials.
- [x] The container serves the SPA and API, runs migrations and passes health
      checks.
- [x] Data survives a container restart when the volume is retained.
- [x] Runtime image contains no compiler, Node.js or pnpm and runs as non-root.
- [x] Backup/restore is documented and tested against a real SQLite database.
- [x] Native binaries build for all six targets and package with checksums.
- [x] The Makefile documents every supported workflow.
- [ ] An actual SSH deployment and a real Alpine/Ubuntu service boot. These need
      an authorized target and are explicitly not performed here.
- [x] No production deployment, push, merge or publish occurs.

## Measurements

See `docs/PROGRESS.md` for the recorded numbers (binary, archive, image size,
startup, idle memory and basic latency) and how they were taken.

## Out of scope

- Kubernetes or autoscaling.
- Multi-replica SQLite.
- Managed database migration.
- Domain/DNS/TLS provisioning.
- Publishing images, archives or releases without authorization.
