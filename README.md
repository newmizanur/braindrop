# BrainDrop

**Capture anything. Organize automatically.**

BrainDrop is an open-source, lightweight, self-hostable personal inbox, task and
reminder manager. Drop in an unstructured thought — *"Buy milk tomorrow, renew my
domain on December 15, and learn Rust async programming"* — and BrainDrop turns
it into structured items you can review, edit and save.

AI is optional. Every core feature works without an API key or any external
service.

## Status

BrainDrop is under active development. **Milestones 1 (bootstrap) and 2 (auth
and shell hardening) are complete**; the application is not production-ready
yet.

| Area | State |
|---|---|
| Loco scaffold, SQLite, SQLite-backed queue | Working |
| Loco JWT authentication | Working, with configurable registration and verification |
| React + Mantine application shell | Working, installable PWA shell |
| Docker-first local CI (`./scripts/ci.sh`) | Working |
| Items, settings, stats | Planned — milestone 3 |
| OpenRouter capture parser | Planned — milestone 4 |
| Telegram / Discord notifications | Planned — milestone 5 |
| Full deployment compose, publishing, backups | Planned — milestone 6 |
| Release hardening | Planned — milestone 7 |

See [`docs/PROGRESS.md`](docs/PROGRESS.md) for the current milestone and
[`docs/tasks/`](docs/tasks/) for the full plan.

## Stack

- Rust (stable) and [Loco](https://loco.rs) 1.3.x
- Axum through Loco, SeaORM, SQLite (WAL)
- Loco native workers and scheduler, SQLite-backed queue
- React 19, TypeScript, Vite, React Router
- [Mantine](https://mantine.dev) 9, Mantine Hooks / Dates / Notifications
- Tabler Icons and TanStack Query
- OpenRouter (optional), Telegram Bot API, Discord webhooks
- Docker and GitHub Actions

No Redis, PostgreSQL, message broker, vector database or microservice is
required.

## Quick start

### Prerequisites

- Rust stable (`rustup`)
- Node.js 20+ and pnpm
- Optional: the Loco generator CLI (`cargo install loco --locked`) and
  `sea-orm-cli` (`cargo install sea-orm-cli --locked`)

### Run locally

```sh
# 1. Install and build the SPA (Loco serves frontend/dist)
pnpm --dir frontend install
pnpm --dir frontend build

# 2. Start the app (runs migrations automatically in development)
cargo loco start
```

Open <http://localhost:5150>. The generated authentication routes live under
`/api/auth`; the React app is served from `/`.

For frontend development with hot reload, run the backend and Vite side by side:

```sh
cargo loco start          # terminal 1, http://localhost:5150
pnpm --dir frontend dev   # terminal 2, http://localhost:5173
```

Vite proxies `/api` to the backend port from `PORT` (default `5150`).

### Create the first account

Public registration is disabled by default. Create the first account with the
administrator task, which marks the account verified:

```sh
cargo loco task admin:create \
  email:admin@example.com \
  name:"Admin" \
  password:"change-me"
```

Set `ALLOW_REGISTRATION=true` to let visitors create their own accounts, and
`REQUIRE_EMAIL_VERIFICATION=true` to require a verified address before login.
Both are read from the environment through `config/<env>.yaml`.

Development email uses SMTP on `localhost:1025`. Run a local catcher such as
Mailpit if you want to receive verification and password-reset mail:

```sh
docker run --rm -p 1025:1025 -p 8025:8025 axllent/mailpit
```

## Commands

| Command | Purpose |
|---|---|
| `cargo loco start` | Run the HTTP server |
| `cargo loco start --worker` | Run a background worker process |
| `cargo loco start --scheduler` | Run the scheduler process |
| `cargo loco start --all` | Run server, worker and scheduler together |
| `cargo loco routes` | List registered routes |
| `cargo loco db migrate` | Apply migrations |
| `cargo loco db entities` | Regenerate SeaORM entities from the schema |
| `cargo loco task <name>` | Run a CLI task |
| `cargo loco doctor` | Diagnose environment and configuration |
| `pnpm --dir frontend build` | Type-check and build the SPA into `frontend/dist` |
| `pnpm --dir frontend lint` | ESLint |
| `pnpm --dir frontend test` | Vitest |
| `pnpm --dir frontend exec playwright test` | Playwright smoke tests |
| `./scripts/ci.sh` | Run the complete Docker CI pipeline |

## Continuous integration

`./scripts/ci.sh` is the single validation entry point. It builds the CI and
production images with `docker-compose.ci.yml` and runs, in order:

1. Rust formatting, Clippy, tests and a clean SQLite migration.
2. Frontend type-check, lint, unit tests and production build.
3. The production Docker image, including `/_health`.
4. Playwright smoke tests against that image.

Cargo caches live in named Docker volumes and the pnpm store in a BuildKit cache
mount, so repeated runs are fast. Test databases are created inside containers;
development and production data are never mounted.

GitHub Actions is intentionally **manual-only** (`workflow_dispatch`) for now
and calls the same script, so local and hosted runs cannot drift.

## Project layout

```text
src/                 Loco backend (controllers, models, workers, tasks, mailers)
migration/           SeaORM migrations
config/              development / test / production configuration
frontend/            React + Mantine SPA (built into frontend/dist)
tests/               Loco request, model, task and worker tests
docker/ci/           Dockerfiles for the CI stages
scripts/             CI and local E2E server scripts
docs/                specification, architecture, progress, deployment, tasks
```

## Documentation

- [`docs/SPEC.md`](docs/SPEC.md) — product and technical specification
- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) — how the system is put together
- [`docs/PROGRESS.md`](docs/PROGRESS.md) — milestone status and handoff notes
- [`docs/DEPLOYMENT.md`](docs/DEPLOYMENT.md) — production deployment plan
- [`docs/tasks/`](docs/tasks/) — milestone-by-milestone implementation plan
- [`AGENTS.md`](AGENTS.md) — contributor and agent guide for this repository

## Security

- All application data is scoped to the authenticated user.
- Never commit `.env` files, database files, tokens or provider credentials.
- Production requires `DATABASE_URL`, `QUEUE_URL`, `JWT_SECRET` and `HOST` to be
  supplied explicitly; there are no production defaults for secrets.
  `JWT_SECRET` must be base64, because Loco decodes it before signing.
- Public registration is disabled by default; the first account is created with
  `admin:create`.
- AI and notification integrations are optional and must fail gracefully.

## License

MIT — see [`LICENSE`](LICENSE).
