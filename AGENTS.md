# Agent guide for this Loco app

This is a [Loco](https://loco.rs) app — **Rails for Rust**. When you are unsure
how something should work here, the answer is almost always "the way Rails does
it." Where Loco diverges, it is because Rust forced it.

## Table of Contents

<!-- toc:start -->
- [Read this first](#read-this-first)
- [The three rules that prevent most mistakes](#the-three-rules-that-prevent-most-mistakes)
- [Before you call it done](#before-you-call-it-done)
- [More](#more)
- [BrainDrop-specific instructions](#braindrop-specific-instructions)
  - [Project](#project)
  - [Technology decisions](#technology-decisions)
  - [Engineering rules](#engineering-rules)
  - [CI rules](#ci-rules)
  - [Project documentation](#project-documentation)
  - [Markdown documentation convention](#markdown-documentation-convention)
  - [Deployment and distribution](#deployment-and-distribution)
  - [Development workflow](#development-workflow)
  - [Git rules](#git-rules)
  - [Context efficiency](#context-efficiency)
<!-- toc:end -->
## Read this first

A complete Loco skill ships with this app at **`.claude/skills/loco/`**, matched
to the exact `loco-rs` version in `Cargo.toml`:

| File | What it gives you |
|---|---|
| `.claude/skills/loco/SKILL.md` | start here — the router, `AppContext`, project layout, CLI |
| `.claude/skills/loco/doctrine.md` | what good Loco code looks like; read before writing any |
| `.claude/skills/loco/api-index.md` | every public `loco_rs` symbol, generated from rustdoc — **check here before guessing an API name** |
| `.claude/skills/loco/recipes/` | how to add a model, endpoint, worker, task, mailer, middleware, auth, tests |

If your tool supports Agent Skills, it will load `SKILL.md` automatically. If
not, read it directly — it is a normal markdown file.

## The three rules that prevent most mistakes

1. **Generate, then edit.** `cargo loco generate <thing>` writes the file *and*
   the wiring. Rust has no autoloading; hand-wiring is how "the handler exists
   but 404s" happens.
2. **Use the batteries.** This app already has an ORM, queue, scheduler, mailer,
   task runner, storage, cache, and test harness. Adding a crate for something
   Loco already does is the most common mistake.
3. **Fat model, slim controller.** Domain logic on the model; handlers parse,
   call a model method, and render.

## Before you call it done

The project contract is the Docker-first pipeline:

```sh
./scripts/ci.sh
```

It runs Rust formatting, Clippy, tests and migrations, the frontend
type-check/lint/unit tests/build, the production image health check and the
Playwright smoke tests. Do not report a milestone as done on a partial run.

If Docker is genuinely unavailable, run the same checks on the host and say so
explicitly in the report:

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

## More

- Docs: <https://loco.rs/docs/>
- Framework agent guide: <https://loco.rs/AGENTS.md>


## BrainDrop-specific instructions

### Project

BrainDrop is an open-source, lightweight, self-hostable personal inbox, task and reminder manager with optional AI integration.

### Technology decisions

- SQLite for database and persistent job queue.
- Native Loco workers and scheduler.
- Loco-generated React SPA with TypeScript and Mantine.
- Native Loco static asset serving.
- OpenRouter for optional AI integration.
- Telegram and Discord for optional notifications.
- Docker and TierHive SSH deployment.
- MIT license.

### Engineering rules

- Follow the original Loco instructions above.
- Read `.claude/skills/loco/SKILL.md` and `doctrine.md` before implementation.
- Consult the local API index and recipes before guessing framework APIs.
- Preserve the existing Loco scaffold.
- Do not introduce unnecessary dependencies or abstractions.
- Authentication is mandatory; public registration is configurable.
- Always enforce authenticated user ownership.
- Never expose credentials or commit secrets.
- All core functionality must work without AI.

### CI rules

- Docker-first: `./scripts/ci.sh` is the primary and complete validation path.
- Keep `docker-compose.ci.yml` and the CI Dockerfiles aligned with the real
  toolchain; do not add Redis, PostgreSQL or npm to the pipeline.
- Cache Docker layers and persistent dependency caches, but never mount a
  development or production database into CI. Test data lives in the container
  or a dedicated ephemeral volume.
- GitHub Actions stays manual-only (`workflow_dispatch`) until explicitly
  re-enabled, and it must call the same scripts as local CI rather than
  duplicating steps.
- A milestone is not complete until the full pipeline passes.

### Project documentation

- `docs/SPEC.md`
- `docs/ARCHITECTURE.md`
- `docs/PROGRESS.md`
- `docs/DEPLOYMENT.md`
- `docs/tasks/`

### Markdown documentation convention

Maintained Markdown files should have a clickable Table of Contents near the
top, with links for their second-level and third-level headings. Use
GitHub-compatible heading anchors, including numeric suffixes for duplicate
heading text. Keep the table synchronized when headings change and validate
relative Markdown links. Do not add tables of contents to generated,
third-party or vendored Markdown files.

### Deployment and distribution

Three supported paths share one binary and one `config/production.yaml`:

- Docker Compose: `docker-compose.yml` (production) is separate from
  `docker-compose.ci.yml` (tests only). The image runs `start --all`.
- Native: `make build-alpine-*`, `make build-linux-*`, `make build-mac`, then
  `./scripts/package.sh <rust-target>`. Archives land in `dist/` with SHA-256
  sidecars.
- SSH: `make deploy-init` / `make deploy-linux` / `make deploy-rollback` call
  `scripts/deploy.sh`. It uses only `ssh` and `scp`; never add `rsync` or a
  remote compiler, never disable host-key verification, never upload a local
  `.env`, and never let a deploy regenerate an existing encryption key.

SQLite backups must use the online backup API through `scripts/backup.sh`; never
copy a live database with `cp`. Restores require explicit confirmation and keep
the previous files.

Do not deploy, push, publish images/archives or start a release without explicit
authorization. GitHub Actions stays manual-only.

### Development workflow

1. Read the current task and PROGRESS.md.
2. Inspect relevant existing implementation.
3. Consult Loco skills and documentation.
4. Implement only the current milestone.
5. Add or update tests.
6. Run `./scripts/ci.sh` and fix every failure.
7. Update project documentation.
8. Commit with the implementing agent (Codex or OpenCode) as co-author.
9. Report results and stop.

### Git rules

- Preserve the existing user's Git author identity.
- Use conventional commit messages.
- Include the implementing agent (Codex or OpenCode) as co-author in every
  implementation commit.
- Do not push, merge or deploy without authorization.

### Context efficiency

- Avoid repeatedly scanning the entire repository.
- Read only files relevant to the current task.
- Avoid unrelated refactoring.
- Keep milestones small and independently testable.
- Use PROGRESS.md for context handoff.
- Report blockers rather than inventing APIs.
