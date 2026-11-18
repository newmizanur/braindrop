# BrainDrop

**Capture anything. Organize automatically.**

BrainDrop is an open-source, lightweight, self-hostable personal inbox, task and
reminder manager. Drop in an unstructured thought — *"Buy milk tomorrow, renew my
domain on December 15, and learn Rust async programming"* — and BrainDrop turns
it into structured items you can review, edit and save.

AI is optional. Every core feature works without an API key or any external
service.

## Status

BrainDrop is under active development. **Milestone 1 (bootstrap) is complete**;
the application is not production-ready yet.

| Area | State |
|---|---|
| Loco scaffold, SQLite, SQLite-backed queue | Working |
| Loco JWT authentication | Working (generated) |
| React + Mantine application shell | Working |
| Items, settings, stats | Planned — milestone 3 |
| OpenRouter capture parser | Planned — milestone 4 |
| Telegram / Discord notifications | Planned — milestone 5 |
| Docker, CI publishing, backups | Planned — milestone 6 |
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

Registration is currently the generated Loco flow. Milestone 2 makes public
registration opt-in (disabled by default) and adds an administrator task.

Until then, either register in the UI or use the Loco task:

```sh
cargo loco task user:create \
  email:admin@example.com \
  name:"Admin" \
  password:"change-me"
```

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

## Project layout

```text
src/                 Loco backend (controllers, models, workers, tasks, mailers)
migration/           SeaORM migrations
config/              development / test / production configuration
frontend/            React + Mantine SPA (built into frontend/dist)
tests/               Loco request, model, task and worker tests
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
- AI and notification integrations are optional and must fail gracefully.

## License

MIT — see [`LICENSE`](LICENSE).
