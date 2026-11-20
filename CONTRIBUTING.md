# Contributing to BrainDrop

Thanks for helping improve BrainDrop. This project is a Loco application with a
React/Mantine frontend, and it follows Rails-style conventions wherever they
apply. Read [AGENTS.md](AGENTS.md) before changing code: it is the short version
of the rules below.

## Table of Contents

<!-- toc:start -->
- [Ground rules](#ground-rules)
- [Getting set up](#getting-set-up)
- [Where things live](#where-things-live)
- [How to make a change](#how-to-make-a-change)
- [Style and checks](#style-and-checks)
- [Commits and pull requests](#commits-and-pull-requests)
- [Reporting security issues](#reporting-security-issues)
- [Code of conduct](#code-of-conduct)
- [License](#license)
<!-- toc:end -->
## Ground rules

- BrainDrop must stay fully functional without AI or external notification
  services. Never make a core flow depend on a provider key.
- Authentication is mandatory. Every item, setting and delivery query is scoped
  to the authenticated user.
- Never commit secrets, `.env` files, database files, `target/`, `node_modules`
  or build artifacts. Never log provider credentials, tokens or webhook URLs.
- Do not push, merge, tag, publish or deploy without explicit authorization.
- GitHub Actions is manual-only (`workflow_dispatch`) by instruction.

## Getting set up

You need Rust stable, Node.js 20+ with pnpm (the repository pins pnpm 10.12.4
through Corepack), and Docker for the full validation pipeline.

```sh
pnpm --dir frontend install
pnpm --dir frontend build     # Loco serves frontend/dist
cargo loco task admin:create email:admin@example.com name:"Admin" password:"change-me"
cargo loco start              # http://localhost:5150
```

Public registration is off by default; create the first account with
`admin:create`. For frontend hot reload, run `pnpm --dir frontend dev` alongside
the backend. Development SMTP defaults to `localhost:1025`; run a local catcher
such as Mailpit if you need verification or reset mail.

Copy `.env.example` to `.env` only if you need local overrides, and keep it out
of Git. The file contains placeholders only — fill secrets locally.

## Where things live

| Area | Location |
|---|---|
| Backend routes, hooks, workers, tasks | `src/app.rs`, `src/controllers/`, `src/workers/`, `src/tasks/` |
| Domain logic and scoped queries | `src/models/` (generated entities in `src/models/_entities/` are never hand-edited) |
| Typed JSON contract | `src/dtos/`; `ts-rs` exports to `frontend/src/bindings/` |
| Optional AI client | `src/ai/` |
| Notification clients, scans and deliveries | `src/notifications/` |
| Security middleware and rate limits | `src/middleware/`, `src/auth_rate_limit.rs` |
| Frontend SPA | `frontend/src/`, built into `frontend/dist` |
| Configuration for every environment | `config/development.yaml`, `config/test.yaml`, `config/production.yaml` |
| Migrations | `migration/` |
| Tests | `tests/`, `frontend/src/**/*.test.*`, `frontend/e2e/` |
| Deployment and release tooling | `Makefile`, `scripts/`, `docs/DEPLOYMENT.md` |

The full design is in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md); the product
contract is in [docs/SPEC.md](docs/SPEC.md).

## How to make a change

1. Read the current task in `docs/tasks/` and the status in
   [docs/PROGRESS.md](docs/PROGRESS.md). Keep changes inside one milestone.
2. For models, controllers, migrations, workers and tasks, run the Loco
   generator first (`cargo loco generate ...`). Rust has no autoloading; the
   generator writes the file *and* its wiring. Hand-wiring is how a handler
   ends up returning 404.
3. Keep controllers thin: parse the request, call one or two model methods,
   render a DTO. Queries and state transitions belong on the model.
4. Use what Loco already ships — ORM, queue, scheduler, cache, mailer, tasks
   and test harness. Do not add a crate for something the framework does.
5. Keep all settings in `config/<env>.yaml` through `ctx.config`. Application
   code must not call `std::env` directly.
6. Never serialize an entity straight to a response, spawn a detached task with
   `tokio::spawn`, or use `.unwrap()` in a handler.
7. Add or update tests beside the change. Authorization, cross-user isolation
   and secret non-disclosure are required, not optional.

## Style and checks

```sh
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test --workspace
pnpm --dir frontend typecheck
pnpm --dir frontend lint
pnpm --dir frontend test
```

The authoritative check is the Docker-first pipeline, which also builds the
production image, checks migrations, persistence, backup/restore and the
Playwright smoke suite:

```sh
./scripts/ci.sh
```

Do not report a change as validated on a partial run. If Docker is genuinely
unavailable, run the host commands above and say so explicitly. CI never needs
real AI, Telegram or Discord credentials; provider calls are mocked.

## Commits and pull requests

- Use conventional commit messages (`feat:`, `fix:`, `docs:`, `test:`,
  `chore:`, ...).
- Preserve the existing Git author identity. Include the implementing agent
  (Codex or OpenCode) as a co-author in implementation commits.
- Keep commits atomic and explain *why* in the commit body when it is not
  obvious.
- Update the relevant docs with the code: `README.md`, `docs/ARCHITECTURE.md`,
  `docs/DEPLOYMENT.md`, `docs/PROGRESS.md` and the task file.
- Re-run `./scripts/ci.sh` after the final edit.

## Reporting security issues

Do not open a public issue for a vulnerability. Follow
[SECURITY.md](SECURITY.md) and email **newmizanur@gmail.com** with the details
so the report can be triaged privately.

## Code of conduct

Participation is governed by [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

## License

Contributions are accepted under the [MIT License](LICENSE).
