# BrainDrop — Project Specification

## 1. Role and objective

Act as a senior Rust and React software engineer responsible for implementing BrainDrop, an open-source, lightweight, self-hostable, AI-enhanced personal inbox, task and reminder management application.

**Tagline:** Capture anything. Organize automatically.

BrainDrop must remain fully functional without AI or external services.

## 2. Mandatory documentation

Use official framework documentation as the source of truth.

- <https://loco.rs/docs/>
- <https://loco.rs/docs/reference/cli/>
- <https://loco.rs/docs/tutorials/your-first-app/>
- <https://loco.rs/docs/how-to/serve-assets/>
- <https://mantine.dev/>
- <https://ui.mantine.dev/>
- <https://openrouter.ai/docs/>

Read the relevant documentation before implementing framework-specific functionality. Do not invent framework APIs.

## 3. Technology stack

- Rust stable
- Loco.rs v1.2.x
- Axum through Loco
- SeaORM
- SQLite with WAL
- Loco SQLite-backed job queue
- Loco scheduler and workers
- Loco-generated React SPA
- TypeScript and Vite
- Mantine UI
- Mantine Hooks
- Mantine Dates
- Mantine Notifications
- Tabler Icons
- TanStack Query
- OpenRouter using Reqwest
- Telegram Bot API
- Discord Webhooks
- Docker and Docker Compose
- GitHub Actions
- MIT license

Do not introduce Redis, PostgreSQL, external message brokers, vector databases or unnecessary microservices.

## 4. Mandatory initialization

Do not manually scaffold the backend.

Inspect existing repository contents and Git status before generating anything. Preserve existing files.

Install the required tools if unavailable:

```bash
cargo install loco --locked
cargo install sea-orm-cli --locked
```

Verify generator compatibility with Loco v1.2.x.

Use the official generator:

```bash
loco new \
  --name braindrop \
  --db sqlite \
  --bg queue-sqlite \
  --assets clientside \
  --allow-in-git-repo
```

Generate in a suitable temporary directory if the repository already exists, and safely integrate the generated files into the repository root.

Avoid nested Git repositories.

Preserve Loco's generated application structure, authentication, configuration, models, CLI aliases, tests and frontend scaffolding.

Use `cargo loco generate` for models, controllers, migrations, workers and tasks when appropriate.

Do not replace Loco's architecture with a custom clean architecture framework.

## 5. Product requirements

BrainDrop allows users to capture unstructured thoughts and organize them into tasks, notes, ideas and reminders.

Example input:

> "Buy milk tomorrow, renew my domain on December 15, and learn Rust async programming."

The optional AI parser generates structured item suggestions. Users can review, edit, remove and save these suggestions.

Manual item creation must always work.

### Core features

- Quick capture
- Inbox
- Task management
- Notes
- Ideas
- Reminders
- Categories
- Tags
- Priority
- Due dates
- Full-text input search
- Filtering and sorting
- Pagination
- Dashboard statistics
- Notification preferences
- Dark and light themes
- Responsive mobile interface

## 6. Authentication

Reuse Loco's generated authentication.

Authentication is mandatory for all application data.

Support multiple users with isolated data.

Public registration is configurable and disabled by default.

Provide a Loco CLI task for creating the initial administrator and additional accounts.

Support configurable email verification.

Suggested environment variables:

```env
ALLOW_REGISTRATION=false
REQUIRE_EMAIL_VERIFICATION=false
```

Do not create a separate authentication implementation.

Ensure every item and settings query is scoped to the authenticated user.

Protect against insecure direct object references.

## 7. HTTP API

Limit the core business API to the following ten endpoints.

| Method | Route | Purpose |
|---|---|---|
| GET | /api/items | List and filter items |
| POST | /api/items | Create one or multiple items |
| GET | /api/items/:id | Retrieve item |
| PATCH | /api/items/:id | Update item |
| DELETE | /api/items/:id | Delete item |
| POST | /api/ai/parse | Parse unstructured input |
| GET | /api/stats | Dashboard statistics |
| GET | /api/settings | Retrieve preferences |
| PATCH | /api/settings | Update preferences |
| POST | /api/notifications/test | Test notification channel |

Existing Loco authentication and monitoring routes are excluded from this limit.

Use typed request and response DTOs, input validation, appropriate HTTP status codes and consistent error responses.

Use Loco's TypeScript DTO generation where supported.

## 8. Database

Preserve all framework-generated tables.

Introduce the following application tables.

### items

- id
- user_id
- title
- content
- type
- category
- tags
- status
- priority
- due_at
- remind_at
- completed_at
- created_at
- updated_at

Item types: task, note, idea, reminder.

Statuses: inbox, pending, completed, archived.

Priorities: low, normal, high, urgent.

### user_settings

- id
- user_id
- timezone
- theme
- AI preferences
- Notification preferences
- Created and updated timestamps

### notification_deliveries

- id
- user_id
- item_id, nullable for digests and overdue summaries
- channel
- notification_type
- deduplication_key
- scheduled_at
- status (`pending`, `processing`, `sent`, `failed`, `cancelled`)
- attempts
- next_attempt_at
- sent_at
- last_error
- Created and updated timestamps

Choose appropriate SQLite-compatible types, indexes, constraints and foreign keys.

Store timestamps in UTC and handle user-local timezone conversion correctly.

Protect integration credentials using authenticated encryption and an externally configured encryption key.

Never return stored secrets in API responses.

## 9. OpenRouter integration

Implement a minimal OpenRouter client.

Requirements:

- Optional AI integration.
- Server-configured or user-provided API key.
- Configurable model.
- Structured JSON output when supported.
- Strict response validation.
- Request timeouts.
- Input and output limits.
- Rate limiting.
- Graceful handling of provider failures.
- No streaming required.
- No agents, RAG or embeddings.

Suggested configuration:

```env
OPENROUTER_API_KEY=
OPENROUTER_MODEL=
OPENROUTER_BASE_URL=https://openrouter.ai/api/v1
```

The AI parser must receive the user's timezone and current date to resolve relative dates.

Do not invent deadlines for items without deadlines.

AI-generated items must be reviewed before being persisted.

Treat user input as untrusted data, not instructions that can override the extraction prompt.

Mock OpenRouter in automated tests.

## 10. Notification integrations

Support Telegram and Discord independently.

Users may enable one, both or neither.

### Telegram

Use the Telegram Bot API with a bot token and chat ID.

### Discord

Use Discord webhooks without maintaining a bot gateway connection.

### Notification types

- Scheduled reminders
- Daily digest
- Overdue tasks
- Test notification (sent directly by `POST /api/notifications/test`, not persisted)

Implement provider-specific clients behind a small common interface.

Add timeouts, bounded retries and failure handling.

Restrict outbound integration destinations to approved provider endpoints to prevent SSRF.

Never log secrets.

## 11. Workers and scheduling

Use native Loco workers and scheduling functionality.

Use persistent SQLite-backed background jobs.

Implement the following scheduled tasks:

- Reminder scan every minute.
- Daily digest eligibility scan every minute.
- Overdue task scan hourly.
- Pending/abandoned delivery recovery every minute.
- Notification history cleanup daily.

The scheduler should identify eligible notifications and enqueue delivery jobs.

Persist notification state and use unique deduplication keys.

Recover pending deliveries after restarts.

Avoid duplicate enqueueing and minimize duplicate external deliveries. Do not claim exactly-once delivery where providers do not support it.

Use one application replica for the initial SQLite deployment.

Verify the supported Loco runtime commands for running the server, workers and scheduler.

## 12. Frontend

Extend the React SPA generated by Loco.

Use Mantine and components inspired by official Mantine UI examples.

Use a clean productivity-oriented design rather than a generic admin template.

Implement:

- Login
- Registration when enabled
- Dashboard
- Inbox
- Today
- Upcoming
- All items
- Completed
- Settings
- Integrations

Use Mantine AppShell for the application layout.

Provide a quick-capture modal and AI item-review interface.

Support mobile navigation, accessible controls, loading states, empty states and error states.

Support light and dark modes.

Add basic PWA installability. Do not cache private API responses or authentication credentials through the service worker.

## 13. Static asset serving

Use Loco's native static asset middleware.

Build the React application into `frontend/dist`.

Serve production frontend assets through the Rust application.

Do not require an independent frontend server, Nginx or Node.js in the production runtime.

Support client-side navigation and direct page refreshes.

Ensure unknown API routes return appropriate API errors rather than SPA HTML.

Embedded binary assets are optional future work and are not required for MVP completion.

## 14. Deployment

Provide:

- Multi-stage Dockerfile
- Docker Compose configuration
- .dockerignore
- .env.example
- Docker-first local CI (`docker-compose.ci.yml`, `scripts/ci.sh`)
- GitHub Actions CI (manual `workflow_dispatch` only for now)
- Native release binaries and distribution archives
- TierHive-compatible SSH deployment
- Backup and restore scripts
- Production deployment documentation

Build the frontend in a Node.js build stage and compile Rust in a separate build stage.

The runtime image should contain only necessary runtime libraries, application files, frontend assets and configuration.

Run as a non-root user.

Store SQLite files in a persistent volume.

Use Loco's built-in monitoring endpoints.

Keep production secrets outside the repository.

TierHive-specific SSH commands and platform capabilities must be verified before activating deployment.

Do not guess deployment credentials or execute production deployment without authorization.

Support safe SQLite backups using a consistent database backup mechanism.

Deployment must also work without Docker on the target. Native builds cover
`x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`,
`x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`,
`aarch64-apple-darwin` and `x86_64-apple-darwin`. Distribution archives contain
the binary, `config/`, `frontend/dist`, operational scripts and documentation,
never a database or a real `.env`.

The SSH flow uses only `ssh` and `scp`, keeps host-key verification enabled,
stages uploads, activates releases atomically, preserves `shared/.env` and data,
health-checks after restart and retains previous releases for rollback without
downgrading the database.

Local CI is Docker-first. One command, `./scripts/ci.sh`, builds the CI and
production images from `docker-compose.ci.yml` and runs the complete validation
suite. Cargo caches live in named Docker volumes and the pnpm store in a BuildKit
cache mount. Test
databases are created inside containers and never point at development or
production data.

Automatic GitHub Actions runs are disabled for now. The workflow is retained for
manual `workflow_dispatch` runs and must call the same scripts as local CI so
the two paths cannot drift. GitHub Actions and local CI should eventually reuse
the same validation scripts; this is already the case for the Docker pipeline.

## 15. Testing and quality

Use native Loco request and model testing conventions.

The single validation command is:

```bash
./scripts/ci.sh
```

It covers:

Backend:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --workspace
cargo loco db migrate   # against a throwaway SQLite database
```

Frontend:

- Type checking
- ESLint
- Vitest
- Production build
- Playwright smoke tests against the built application

Deployment:

- Multi-stage production Docker image build
- Application start with a disposable database
- `/_health` check and a seeded administrator login
- Create-item / container-restart persistence probe
- In-container SQLite backup
- Backup/restore self-test, deploy argument validation, service-script syntax
  and distribution package layout checks

Test authorization, cross-user isolation, AI parsing, notification delivery, scheduler deduplication, restart recovery and disabled public registration.

Verify a clean Docker build and persistence across container restarts.

CI must not require real external API credentials.

## 16. Repository and Git

Use the existing Git author identity.

Add Codex as co-author to every implementation commit using its appropriate configured co-author identity.

Never fabricate the user's Git identity.

Create meaningful atomic commits.

Use conventional commit messages.

Do not commit:

- Secrets
- Database files
- Build artifacts
- node_modules
- Rust target directory

Do not push, merge or deploy unless authorized.

## 17. Task management

Create the following project documentation:

```text
AGENTS.md
README.md
docs/
  SPEC.md
  ARCHITECTURE.md
  PROGRESS.md
  DEPLOYMENT.md
  tasks/
    001-bootstrap.md
    002-auth-and-shell.md
    003-items.md
    004-ai.md
    005-notifications.md
    006-deployment.md
    007-release.md
```

Each task must include:

- Objective
- Scope
- Dependencies
- Implementation checklist
- Acceptance criteria
- Validation commands
- Explicit out-of-scope items

Work on only one milestone at a time.

Update PROGRESS.md after each milestone.

## 18. Initial execution instructions

Start with Task 001 only.

1. Inspect the repository and installed tools.
2. Review official Loco documentation.
3. Create the project documentation and AGENTS.md.
4. Generate the application using loco new.
5. Configure SQLite and the native SQLite queue.
6. Verify the generated authentication.
7. Extend the generated React frontend with Mantine.
8. Implement a minimal BrainDrop application shell.
9. Verify backend and frontend builds.
10. Run applicable tests.
11. Commit the validated milestone without co-author.
12. Report results, blockers, decisions and the next task.

Do not proceed to Task 002 automatically.

Prioritize working, maintainable software over unnecessary abstractions.
