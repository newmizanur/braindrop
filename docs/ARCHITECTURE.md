# BrainDrop Architecture

This document describes how BrainDrop is put together, what is implemented today,
and the design constraints for the remaining milestones. It is a living document:
update it when a milestone changes the design.

## 1. Status

Milestones 1 (bootstrap) and 2 (auth and shell hardening) are complete:

- Loco application scaffold, generated with SQLite, a SQLite-backed queue and a
  client-side React SPA.
- Loco's generated JWT authentication, users model, mailers, tasks and tests are
  preserved.
- SQLite is the application database and the queue backend. Loco enables WAL for
  file-backed SQLite connections.
- The frontend has been extended with Mantine, a responsive `AppShell`, theme
  switching, authentication screens and placeholder pages for the product areas.
- Unknown `/api/*` routes are answered by a JSON 404 catch-all, so a mistyped API
  path never falls through to the SPA.
- Authentication is production-shaped: registration and email verification are
  configurable, `admin:create` provisions the first account, and the SPA
  validates its token against `/api/auth/current`.
- The SPA is an installable PWA whose service worker caches only the app shell.
- `./scripts/ci.sh` runs the full Docker-based validation suite, including a
  production image build and Playwright smoke tests.

Items, settings, statistics, AI, notifications, full deployment artifacts and
release hardening are **not implemented yet**. See `docs/PROGRESS.md` and
`docs/tasks/`.

## 2. Design principles

1. **Loco first.** Use Loco's ORM, queue, scheduler, tasks, mailers, auth and
   static serving. Do not replace them with a custom framework.
2. **Fat model, slim controller.** Queries and state transitions belong on the
   model; handlers parse, call the model and render a DTO.
3. **Generate, then edit.** Use `cargo loco generate` for models, controllers,
   migrations, workers and tasks so the wiring is generated with the file.
4. **AI is optional.** Manual capture and all core flows must work with no AI
   provider configured.
5. **One user's data never leaks to another.** Every item and settings query is
   scoped by the authenticated user's id, never by a client-supplied user id.
6. **Single-node by default.** The initial deployment is one application replica
   backed by one SQLite database and one SQLite queue. Scaling out is out of
   scope for the MVP.

## 3. Runtime shape

BrainDrop is one Rust binary built from the Loco scaffold. Loco can run it in
several modes:

```text
                    ┌──────────────────────────────┐
  browser ────────▶ │  Loco HTTP server (Axum)      │
                    │  /api/*  +  frontend/dist     │
                    └──────────────┬───────────────┘
                                   │
                    ┌──────────────▼───────────────┐
                    │  SQLite (WAL)                │
                    │  app tables + Loco job queue │
                    └───────┬──────────────┬───────┘
                            │              │
              ┌─────────────▼───┐   ┌──────▼────────────┐
              │ Worker process  │   │ Scheduler process │
              │ delivery jobs   │   │ periodic scans    │
              └─────────────────┘   └───────────────────┘
```

| Mode | Command | Responsibility |
|---|---|---|
| Server | `cargo loco start` | HTTP API, static SPA assets |
| Worker | `cargo loco start --worker` | Consume queued delivery jobs |
| Scheduler | `cargo loco start --scheduler` | Run scheduled scans/tasks |
| All | `cargo loco start --all` | Single-process development / small deploy |

Production uses separate worker and scheduler processes where possible, or
`--all` for a single-container deployment. One replica only for SQLite.

The scheduler does not hold work itself: it shells out to a Loco task on a cron
expression. Recurring work is therefore a `Task` plus a `scheduler:` entry in
`config/*.yaml`. Work that must outlive an HTTP request is a `BackgroundWorker`
on the SQLite queue.

## 4. Directory map

```text
src/
  app.rs                    Hooks: routes, workers, tasks, seeds
  auth_policy.rs            Typed registration / verification policy from config
  controllers/              thin HTTP handlers + route tables
    auth.rs                 generated Loco auth (register, login, JWT, ...)
    api_fallback.rs         JSON 404 for unknown /api routes
  models/
    _entities/              generated SeaORM entities (never hand-edit)
    users.rs                generated user domain logic
  dtos/                     typed JSON DTOs (ts-rs exports to frontend/src/bindings)
  views/                    response types for the generated auth endpoints
  workers/                  Loco BackgroundWorker implementations
  tasks/                    CLI and scheduled tasks
    admin_create.rs         verified initial-administrator task
  mailers/                  generated Loco mailers
migration/                  SeaORM migrations
config/
  development.yaml          local defaults, SQLite files, static frontend/dist
  test.yaml                 isolated test DB, stub mailer, blocking workers
  production.yaml           required secrets, no unsafe defaults
frontend/
  src/
    components/             AppShell, auth layout, shared UI
    pages/                  Dashboard, Inbox, Today, Upcoming, All, Completed, Settings, Integrations
    auth/                   login, register, token storage, route guard
    api/                    typed fetch client and query hooks
    bindings/               generated TypeScript DTOs
  public/                   manifest, icons, service worker
  e2e/                      Playwright smoke tests
docker/
  ci/                       Dockerfiles for the Rust, frontend and Playwright stages
scripts/                    ci.sh (single CI entry point) and helper scripts
tests/                      Loco request/model/task tests
docs/                       specification, architecture, progress, deployment, tasks
```

## 5. Data model

### 5.1 Current tables

Only the framework-generated `users` table exists today. Loco's auth, mailer and
task code depend on it; it must not be replaced.

### 5.2 Planned application tables

These are introduced in milestone 3 (items/settings) and milestone 5
(notifications). Types are SQLite-compatible SeaORM types; timestamps are UTC.

#### `items`

| Column | Type | Notes |
|---|---|---|
| `id` | integer PK | auto-increment |
| `user_id` | integer FK → `users.id` | indexed, cascade delete |
| `title` | string | required, trimmed, length-limited |
| `content` | text, nullable | free-form body |
| `type` | string | `task`, `note`, `idea`, `reminder` |
| `category` | string, nullable | indexed, user-defined |
| `tags` | JSON/text | normalized list, indexed for filtering |
| `status` | string | `inbox`, `pending`, `completed`, `archived` |
| `priority` | string | `low`, `normal`, `high`, `urgent` |
| `due_at` | timestamp, nullable | UTC |
| `remind_at` | timestamp, nullable | UTC |
| `completed_at` | timestamp, nullable | UTC |
| `created_at` | timestamp | UTC |
| `updated_at` | timestamp | UTC |

Indexes: `(user_id, status)`, `(user_id, due_at)`, `(user_id, remind_at)`,
`(user_id, type)`, and a search index or FTS5 table if input search needs it.
Every read and write includes `user_id = authenticated_user.id`.

#### `user_settings`

| Column | Type | Notes |
|---|---|---|
| `id` | integer PK | |
| `user_id` | integer FK → `users.id` | unique, cascade delete |
| `timezone` | string | IANA name, default `UTC` |
| `theme` | string | `system`, `light`, `dark` |
| `ai_enabled` | boolean | default false |
| `ai_model` | string, nullable | optional per-user override |
| `ai_api_key_ciphertext` | text, nullable | authenticated encryption |
| `notification_preferences` | JSON | channel toggles and digest time |
| `telegram_chat_id` | string, nullable | not secret |
| `telegram_token_ciphertext` | text, nullable | authenticated encryption |
| `discord_webhook_ciphertext` | text, nullable | authenticated encryption |
| `created_at` | timestamp | UTC |
| `updated_at` | timestamp | UTC |

Secrets are encrypted with an externally configured key and are never returned by
the API. The API returns only booleans such as `telegram_configured`.

#### `notification_deliveries`

| Column | Type | Notes |
|---|---|---|
| `id` | integer PK | |
| `user_id` | integer FK → `users.id` | indexed |
| `item_id` | integer FK → `items.id`, nullable | null for digests |
| `channel` | string | `telegram`, `discord` |
| `notification_type` | string | `reminder`, `daily_digest`, `overdue`, `test` |
| `deduplication_key` | string | unique per intended delivery |
| `scheduled_at` | timestamp | UTC |
| `status` | string | `pending`, `processing`, `sent`, `failed` |
| `attempts` | integer | default 0 |
| `next_attempt_at` | timestamp, nullable | UTC |
| `sent_at` | timestamp, nullable | UTC |
| `last_error` | text, nullable | sanitized, no secrets |
| `created_at` | timestamp | UTC |
| `updated_at` | timestamp | UTC |

Unique index on `deduplication_key`; indexes on `(status, next_attempt_at)` and
`(user_id, created_at)`. The unique key prevents the scheduler from enqueueing the
same reminder twice. Delivery is at-least-once in intent; providers do not offer
exactly-once semantics and the code must not claim it.

### 5.3 Ownership rules

- The authenticated user's database id is resolved from the JWT `pid` claim.
- Item and settings lookups use both the item/settings id **and** the user id.
- A missing row and a row owned by someone else both return 404, so the API does
  not confirm the existence of another user's data.
- Bulk operations resolve each item through the same scoped finder.

## 6. HTTP API

The core business API is limited to ten endpoints (milestones 3–5):

| Method | Route | Auth | Purpose |
|---|---|---|---|
| GET | `/api/items` | JWT | List, filter, sort, paginate |
| POST | `/api/items` | JWT | Create one or many items |
| GET | `/api/items/:id` | JWT | Retrieve one owned item |
| PATCH | `/api/items/:id` | JWT | Update one owned item |
| DELETE | `/api/items/:id` | JWT | Delete one owned item |
| POST | `/api/ai/parse` | JWT | Parse text into reviewable suggestions |
| GET | `/api/stats` | JWT | Dashboard counts |
| GET | `/api/settings` | JWT | Retrieve preferences (no secrets) |
| PATCH | `/api/settings` | JWT | Update preferences |
| POST | `/api/notifications/test` | JWT | Send a test notification |

The generated Loco auth routes (`/api/auth/*`) and monitoring routes (`/_ping`,
`/_health`, `/_readiness`) are outside this limit.

Conventions:

- JSON request and response bodies, typed DTOs in `src/dtos/`.
- `ts-rs` exports TypeScript bindings into `frontend/src/bindings/`.
- Validation is declared on DTOs/models and returns 400/422 with a consistent
  error body.
- Not found and not-owned return 404; unauthenticated returns 401.
- No entity is serialized directly; handlers render DTOs.
- Unknown `/api/*` routes return JSON errors, never the SPA HTML.

## 7. Authentication and authorization

BrainDrop uses Loco's generated JWT authentication, extended with a small policy
layer:

- `POST /api/auth/register` is gated by `ALLOW_REGISTRATION`. When disabled it
  returns `403` with the code `registration_disabled`.
- `POST /api/auth/login` returns a JWT. When `REQUIRE_EMAIL_VERIFICATION` is on
  and the account is unverified, it returns `403` with
  `email_verification_required`.
- `GET /api/auth/config` returns the two public policy booleans and no user
  data. The SPA uses it to hide the register link and route.
- `GET /api/auth/current` returns the current user. The SPA validates its stored
  token through it before rendering protected routes.
- Verification, forgot/reset and magic-link flows remain available.
- `auth::JWT` is the extractor on every protected handler.

Policy is a typed struct (`src/auth_policy.rs`) deserialized from `settings.auth`
via `Config::settings<T>()`. Handlers never read environment variables directly.
The `admin:create` task is the supported way to create the first account; it
marks the account verified unless `verified:false` is passed and requires
`force:true` to reset an existing account.

No parallel auth implementation, session store or password scheme is introduced.
JWT signing failures return `500`, not `401`, because they indicate a server
misconfiguration. Loco base64-decodes `JWT_SECRET`, so it must be base64.

## 8. OpenRouter integration

Milestone 4 adds a small server-side client:

- Config: `OPENROUTER_API_KEY`, `OPENROUTER_MODEL`, `OPENROUTER_BASE_URL`.
- `reqwest` with connect/request timeouts and a bounded response body.
- Structured JSON output when the model/provider supports it; strict schema
  validation afterward.
- The prompt receives the user's timezone and the current date so relative dates
  ("tomorrow", "December 15") resolve correctly.
- Items without a deadline get `due_at = null`; the model is instructed not to
  invent dates.
- User text is delimited and treated as untrusted data, not instructions.
- Suggestions are returned to the client and are **not persisted** until the user
  reviews and saves them through `POST /api/items`.
- Rate limiting per user, input length limits and output item limits.
- Provider failures return a typed, user-safe error; manual capture still works.
- Tests mock the provider; CI needs no real API key.

## 9. Notifications

Milestone 5 adds Telegram and Discord behind a small channel interface:

```text
NotificationChannel
  send(message) -> Result<DeliveryOutcome>
```

- Telegram: Bot API `sendMessage`, token + chat id.
- Discord: webhook POST, no gateway connection.
- Both use timeouts, bounded retries and sanitized errors.
- Destinations are restricted to the approved provider endpoints (no arbitrary
  URLs, no user-supplied host), which prevents SSRF.
- Credentials are encrypted at rest and never returned by the API or logged.
- A `NotificationDeliveryWorker` consumes queued deliveries.
- A unique `deduplication_key` plus status transitions prevents duplicate
  enqueueing; restart recovery requeues stale `processing` rows.

Scheduled scans:

| Schedule | Task | Purpose |
|---|---|---|
| every minute | reminder scan | enqueue due reminders |
| every minute | digest eligibility | enqueue daily digests at user-local time |
| hourly | overdue scan | enqueue overdue task notifications |
| daily | cleanup | delete old delivery history |

## 10. Frontend

The SPA is built by Vite into `frontend/dist` and served by Loco's static
middleware with an `index.html` fallback for client-side routes.

- Mantine `AppShell` with a header, responsive navbar, burger on mobile, theme
  toggle and user menu.
- Routes: login, register, dashboard, inbox, today, upcoming, all items,
  completed, settings, integrations. The register route is only mounted when
  the public auth config allows registration.
- TanStack Query for server state; the typed fetch client adds the JWT and
  clears it on 401.
- `RequireAuth` validates the stored token with `/api/auth/current` before
  rendering protected routes, showing a full-page loader while it checks and
  clearing token + query cache when validation fails.
- Mantine Notifications for transient feedback.
- Loading, empty and error states are part of each page, not an afterthought.
- Light/dark/system theme, persisted by Mantine; the initial color scheme is set
  before paint to avoid a flash.
- Quick capture is a modal in the shell. In milestone 3 it calls `POST /api/items`;
  in milestone 4 it can call `/api/ai/parse` and open the review UI.
- PWA installability is implemented with `manifest.webmanifest`, generated
  icons and a minimal service worker. The worker caches only the app shell and
  static assets; `/api/*`, `/_*` and all non-GET requests bypass the cache. It is
  registered in production builds only, so development never serves stale
  assets.

## 11. Static asset serving

`config/*.yaml` points Loco's static middleware at `frontend/dist` with
`fallback: frontend/dist/index.html`. This provides:

- production assets without Node.js or Nginx at runtime;
- direct refreshes of client-side routes;
- API requests still handled by Axum before the SPA fallback.

Unknown API routes are answered by `src/controllers/api_fallback.rs`, an
`/api/{*rest}` catch-all that returns `404 application/json` with the shared
`ApiError` shape. This was implemented and smoke-tested in milestone 1 so a
mistyped API path never receives `index.html` with a 200.

## 12. Configuration and secrets

All settings live in `config/<env>.yaml` and are read through `ctx.config`.
Environment variables are interpolated by Loco (`get_env`), not read ad hoc in
handlers.

| Variable | Purpose | Required |
|---|---|---|
| `DATABASE_URL` | SQLite app database | production |
| `QUEUE_URL` | SQLite queue database | production |
| `JWT_SECRET` | JWT signing key | production |
| `HOST` | public base URL | production |
| `ALLOW_REGISTRATION` | enable public registration | no (default false in milestone 2) |
| `REQUIRE_EMAIL_VERIFICATION` | require verified email to log in | no |
| `OPENROUTER_API_KEY` | optional AI provider key | no |
| `OPENROUTER_MODEL` | optional model id | no |
| `OPENROUTER_BASE_URL` | provider base URL | no |
| `SECRETS_KEY` | encryption key for integration credentials | production, if integrations used |
| `MAILER_*` | SMTP settings | production |

Development config contains a generated JWT secret and local SQLite defaults;
those are not production secrets. Production config has no defaults for secrets
and refuses to boot without them.

## 13. Security

- Authenticated user ownership on every data query.
- 404 (not 403) for another user's resources.
- No secrets in API responses, logs or error messages.
- Authenticated encryption for provider credentials.
- SSRF-resistant outbound destinations.
- Input limits on AI text, titles, tags and search.
- CSRF is not a concern for bearer-token API calls, but the frontend stores the
  JWT in `localStorage`; XSS prevention and a strict CSP are part of hardening.
- Service worker must not cache `/api/*` or auth credentials.
- Dependencies are kept current; CI runs `cargo clippy` and frontend lint/tests.

## 14. Testing

Backend uses Loco's request/model testing conventions:

- Auth: registration, login, current user, verification.
- Registration/verification policy, including `admin:create` idempotency.
- Authorization and cross-user isolation for every item/settings endpoint.
- Validation and error shapes.
- AI parsing with a mocked OpenRouter.
- Notification delivery with mocked providers.
- Scheduler deduplication and restart recovery.
- Disabled public registration.

Frontend:

- TypeScript type-check and production build.
- ESLint.
- Vitest for utilities/components.
- Playwright smoke tests for protected-route redirect, registration visibility,
  login → dashboard → logout, bad-credential handling, the PWA manifest and
  service-worker registration (including that no `/api/*` path is cached).

CI never requires real external credentials.

### 14.1 Continuous integration

`./scripts/ci.sh` is the single validation entry point and is Docker-first:

```text
docker-compose.ci.yml
  rust        fmt, clippy, cargo test, `db migrate` on a throwaway database
  frontend    typecheck, lint, Vitest, production build
  app         production image, `/api/auth/config`, seeded login, `/_health`
  e2e         Playwright against the app container's network namespace
```

Cargo caches live in named Docker volumes and the pnpm store in a BuildKit cache
mount. Test databases are created
inside containers and never mount development or production data. The
Playwright stage shares the app container's network namespace so Chromium can
use `http://127.0.0.1:5150`; Chromium otherwise upgrades non-localhost HTTP
origins to HTTPS and fails.

GitHub Actions is manual-only (`workflow_dispatch`) and calls the same script, so
local and hosted checks cannot drift.

## 15. Deployment

Milestone 2 added the production image and CI harness:

- Multi-stage `Dockerfile` (pnpm frontend build, Rust release build, slim
  non-root runtime with `frontend/dist` and `config/`).
- `.dockerignore`, `docker-compose.ci.yml`, `docker/ci/*.Dockerfile` and
  `scripts/ci.sh`.

Milestone 6 still adds:

- `docker-compose.yml` for real deployments and `.env.example`.
- Persistent volume wiring for both SQLite files.
- GitHub Actions publishing to GHCR (the workflow exists but is manual-only).
- TierHive-compatible SSH deployment template (verified before activation).
- Backup/restore scripts using SQLite's online backup mechanism.
- Production deployment documentation.

Single replica for SQLite. Health checks use Loco's `/_health` and `/_readiness`.

## 16. Decisions and deviations

| Decision | Rationale |
|---|---|
| **Loco 1.3.1 instead of 1.2.x** | The repository was delivered with a complete Loco 1.3.1 scaffold (SeaORM 2, React 19, React Router 8, ts-rs 12). Regenerating or downgrading would discard a working, current scaffold and risk breaking generated auth/tests. The architecture and conventions are the same as 1.2.x; the docs are written against the 1.2.x API surface, and 1.3.1 remains compatible. |
| **Scaffold preserved, not regenerated** | `loco new` had already produced the correct structure: SQLite DB, `queue: kind: Sqlite`, `--assets clientside` frontend, auth, tests and config. Re-running it would overwrite user work and create nested state. |
| **Mantine 9 + React Router 8** | Current versions installed by the generator/ecosystem; APIs used are the documented Mantine 9 `AppShell`/provider APIs. |
| **pnpm for the frontend** | The generated `package.json` declares pnpm; CI uses pnpm 10.12.4 through Corepack, and the frontend image installs from the frozen lockfile. |
| **Docker-first CI** | One command (`./scripts/ci.sh`) validates Rust, frontend, the production image and Playwright. It removes host-toolchain drift and exercises the artifact that actually deploys. GitHub Actions calls the same script, manual-only for now. |
| **Base64 `JWT_SECRET`** | Loco base64-decodes the secret before signing. Production config and `.env.example` document this, and signing failures return `500` instead of a misleading `401`. |
| **SQLite WAL** | Loco 1.3 enables WAL automatically for file-backed SQLite connections; no custom PRAGMA code is needed. |
| **Single application replica** | SQLite and the SQLite queue are single-writer; horizontal scaling is out of scope. |
| **JWT in localStorage** | Generated Loco frontend behavior. CSP/XSS review and optional cookie mode remain release hardening (milestone 7). |

## 17. Open questions

- Exact FTS strategy for input search: FTS5 virtual table vs. indexed `LIKE`.
- Whether AI credentials are server-wide, per-user, or both (spec allows both).
- Digest scheduling granularity and default send time.
- TierHive runtime capabilities and the exact SSH deployment contract.
