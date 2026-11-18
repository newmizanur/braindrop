# Agent guide for this Loco app

This is a [Loco](https://loco.rs) app — **Rails for Rust**. When you are unsure
how something should work here, the answer is almost always "the way Rails does
it." Where Loco diverges, it is because Rust forced it.

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

```sh
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test
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

### Project documentation

- `docs/SPEC.md`
- `docs/ARCHITECTURE.md`
- `docs/PROGRESS.md`
- `docs/DEPLOYMENT.md`
- `docs/tasks/`

### Development workflow

1. Read the current task and PROGRESS.md.
2. Inspect relevant existing implementation.
3. Consult Loco skills and documentation.
4. Implement only the current milestone.
5. Add or update tests.
6. Run formatting, linting, tests and applicable build checks.
7. Update project documentation.
8. Commit with Codex as co-author.
9. Report results and stop.

### Git rules

- Preserve the existing user's Git author identity.
- Use conventional commit messages.
- Include Codex as co-author in every implementation commit.
- Do not push, merge or deploy without authorization.

### Context efficiency

- Avoid repeatedly scanning the entire repository.
- Read only files relevant to the current task.
- Avoid unrelated refactoring.
- Keep milestones small and independently testable.
- Use PROGRESS.md for context handoff.
- Report blockers rather than inventing APIs.
