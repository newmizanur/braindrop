# BrainDrop Architecture

This document describes how BrainDrop is put together, what is implemented today,
and the design constraints for the remaining milestones. It is a living document:
update it when a milestone changes the design.

## 1. Status

Milestones 1 (bootstrap), 2 (auth and shell hardening), 3 (items and settings),
4 (AI capture), 5 (notifications) and 6 (deployment and distribution) are
complete. Task 007 release preparation is implemented and awaiting final CI:

- Loco application scaffold, generated with SQLite, a SQLite-backed queue and a
  client-side React SPA.
- Loco's generated JWT authentication, users model, mailers, tasks and tests are
  preserved.
- SQLite is the application database and the queue backend. Loco enables WAL for
  file-backed SQLite connections.
- The frontend has been extended with Mantine, a responsive `AppShell`, theme
  switching, authentication screens, item views and settings.
- Unknown `/api/*` routes are answered by a JSON 404 catch-all, so a mistyped API
  path never falls through to the SPA.
- Authentication is production-shaped: registration and email verification are
  configurable, `admin:create` provisions the first account, the SPA validates
  its token against `/api/auth/current`, and auth endpoints have in-process rate
  limits.
- The SPA is an installable PWA whose service worker caches only the app shell.
- Items, settings, statistics and optional AI-assisted capture are implemented.
  AI suggestions are reviewed in the quick-capture dialog and are only persisted
  when the user saves them.
- Telegram/Discord notifications use durable delivery rows, deduplication and
  five scheduled scans; external providers are mocked in tests and CI.
- Deployment artifacts include Docker Compose, six native targets, an SSH
  release flow, and online SQLite backup/restore.
- Task 007 adds explicit security headers, a no-store cache policy for private
  responses, auth rate limits with a configurable trusted proxy source, a 1 MiB
  body limit, input validation, disabled OpenRouter redirects, redacted
  notification errors and hardened SSH staging. Release docs live in
  `docs/releases/`.
- `./scripts/ci.sh` runs the full Docker-based validation suite, including a
  production image build and Playwright smoke tests. The AI Playwright flow uses
  a local mock provider, so CI needs no real credentials.

Notifications, deployment artifacts and the v0.1.0 documentation are therefore
implemented; the final `./scripts/ci.sh` run, tagging, publishing and deployment
are pending explicit authorization. See `docs/PROGRESS.md` and `docs/tasks/`.

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
  app.rs                    Hooks: routes, middleware, workers, tasks, seeds
  auth_policy.rs            Typed registration / verification policy from config
  auth_rate_limit.rs        In-process limits for authentication endpoints
  middleware/
    cache_control.rs        no-store for API, monitoring and HTML responses
  ai/                       optional OpenRouter client, prompt, validation, credentials
    client.rs               provider trait + reqwest chat-completions client
    config.rs               typed `settings.ai` access and bounds
    credentials.rs          AES-256-GCM sealing for personal provider keys
    prompt.rs               delimited untrusted input + response schema
    suggestions.rs          strict model-output validation
    availability.rs         server vs personal credential resolution
    rate_limit.rs           per-user cache-backed limiter
  controllers/              thin HTTP handlers + route tables
    auth.rs                 generated Loco auth (register, login, JWT, ...)
    ai.rs                   POST /api/ai/parse
    api_fallback.rs         JSON 404 for unknown /api routes
  models/
    _entities/              generated SeaORM entities (never hand-edit)
    users.rs                generated user domain logic
    items.rs                item domain logic and scoped queries
    user_settings.rs        preferences and AI credential state
    stats.rs                dashboard counts
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
  public/                   manifest, icons, service worker, external color-scheme script
  e2e/                      Playwright smoke tests
docker/
  ci/                       Dockerfiles for the Rust, frontend and Playwright stages
scripts/                    ci.sh (single CI entry point) and helper scripts
tests/                      Loco request/model/task tests
docs/                       specification, architecture, progress, deployment, tasks, releases
```

## 5. Data model

### 5.1 Current tables

`users`, `items`, `user_settings` and `notification_deliveries` exist today.
Loco's auth, mailer and task code depend on `users`; it must not be replaced.
Items and settings were implemented in milestone 3 and extended with AI
preferences in milestone 4 and notification preferences/delivery in milestone 5.

### 5.2 Application tables

Types are SQLite-compatible SeaORM types; timestamps are UTC.

#### `items`

| Column | Type | Notes |
|---|---|---|
| `id` | integer PK | auto-increment, never exposed |
| `pid` | UUID | public identifier, unique |
| `user_id` | integer FK → `users.id` | indexed, cascade delete |
| `title` | string | required, trimmed, length-limited |
| `content` | text, nullable | free-form body |
| `item_type` | string | `task`, `note`, `idea`, `reminder`; exposed as `type` |
| `category` | string, nullable | indexed, user-defined |
| `tags` | JSON | normalized list, filtered with `LIKE` |
| `status` | string | `inbox`, `pending`, `completed`, `archived` |
| `priority` | string | `low`, `normal`, `high`, `urgent` |
| `due_at` | timestamp, nullable | UTC |
| `remind_at` | timestamp, nullable | UTC |
| `completed_at` | timestamp, nullable | UTC |
| `archived_at` | timestamp, nullable | UTC |
| `created_at` | timestamp | UTC |
| `updated_at` | timestamp | UTC |

Indexes cover `user_id` plus `status`, `due_at`, `remind_at`, `item_type`,
`created_at`, `completed_at`, `priority` and `category`. Every read and write
includes `user_id = authenticated_user.id`.

#### `user_settings`

| Column | Type | Notes |
|---|---|---|
| `id` | integer PK | |
| `user_id` | integer FK → `users.id` | unique, cascade delete |
| `timezone` | string | IANA name, default `UTC` |
| `theme` | string | `auto`, `light`, `dark` |
| `default_item_type` | string | default `task` |
| `default_priority` | string | default `normal` |
| `week_start` | integer | `0` Sunday … `6` Saturday, default `1` |
| `ai_enabled` | boolean | account-level AI switch, default true |
| `ai_use_server_key` | boolean | prefer the server key over a personal one |
| `ai_model` | string | account model override; empty means the server default |
| `ai_api_key_encrypted` | text, nullable | AES-256-GCM envelope; never returned |
| `notifications_enabled` | boolean | master switch, default false |
| `telegram_enabled` | boolean | default false |
| `telegram_chat_id` | string | destination chat; safe to display |
| `telegram_bot_token_encrypted` | text, nullable | AES-256-GCM envelope; never returned |
| `discord_enabled` | boolean | default false |
| `discord_webhook_encrypted` | text, nullable | AES-256-GCM envelope; never returned |
| `reminders_enabled` | boolean | default false |
| `daily_digest_enabled` | boolean | default false |
| `daily_digest_time` | string | local `HH:MM`, default `09:00` |
| `overdue_notifications_enabled` | boolean | default false |
| `created_at` | timestamp | UTC |
| `updated_at` | timestamp | UTC |

AI preferences live in `user_settings`. The server key, model and base URL stay
in configuration, not in the database. A personal OpenRouter key is sealed as
`v1:<base64url(nonce || ciphertext || tag)>` with a key derived from
`AI_ENCRYPTION_KEY`. The API exposes only `has_personal_key`, the resolved
status and the credential source.

Notification preferences also live in `user_settings`. Every notification switch
defaults to off. The Telegram bot token and Discord webhook are sealed with the
same envelope using `NOTIFICATION_ENCRYPTION_KEY` (falling back to
`AI_ENCRYPTION_KEY` when empty). The API exposes `enabled`, `configured`,
`token_set`, `webhook_set` and the non-secret Telegram chat id only.

#### `notification_deliveries`

| Column | Type | Notes |
|---|---|---|
| `id` | integer PK | |
| `user_id` | integer FK → `users.id` | indexed |
| `item_id` | integer FK → `items.id`, nullable | null for digests/overdue; `SET NULL` on item delete |
| `channel` | string | `telegram`, `discord` |
| `notification_type` | string | `reminder`, `daily_digest`, `overdue` |
| `deduplication_key` | string | unique per intended delivery |
| `scheduled_at` | timestamp | UTC |
| `status` | string | `pending`, `processing`, `sent`, `failed`, `cancelled` |
| `attempts` | integer | default 0 |
| `next_attempt_at` | timestamp, nullable | UTC |
| `sent_at` | timestamp, nullable | UTC |
| `last_error` | text, nullable | sanitized, no secrets |
| `created_at` | timestamp | UTC |
| `updated_at` | timestamp | UTC |

Unique index on `deduplication_key`; indexes on `(status, next_attempt_at)`,
`(status, updated_at)`, `(user_id, channel)`, `item_id` and `scheduled_at`.
The unique key prevents the scheduler from enqueueing the same reminder twice.
`POST /api/notifications/test` sends directly and is not persisted. Delivery is
at-least-once in intent; providers do not offer exactly-once semantics and the
code must not claim it.

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
  error body. Invalid enum values return 422.
- Not found and not-owned return 404; unauthenticated returns 401.
- Generated auth endpoints return 429 with code `rate_limited` when the
  in-process account/client budget is exhausted.
- No entity is serialized directly; handlers render DTOs.
- `POST /api/items` accepts one item or `{ "items": [...] }` and inserts the
  batch in a single transaction. `GET /api/items` filters by `q`, `type`,
  `status` (comma-separated), `priority`, `category`, `tag`, `due_from` and
  `due_to`, with `sort`, `order`, `page` and `page_size`.
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

Milestone 4 implements a small server-side client behind an `AiProvider` trait:

- Config: `AI_ENABLED`, `OPENROUTER_API_KEY`, `OPENROUTER_MODEL`,
  `OPENROUTER_BASE_URL`, `AI_TIMEOUT_SECONDS`, `AI_MAX_INPUT_CHARS`,
  `AI_RATE_LIMIT_PER_MINUTE` and `AI_ENCRYPTION_KEY`.
- `reqwest` with rustls, connect/request timeouts, a bounded response body and
  redirects disabled, so a secret-bearing request cannot be redirected to
  another host by the configured base URL.
- Structured JSON output (`json_schema`) when the model supports it. If a model
  rejects `response_format`, the client retries once without it; the system
  prompt still requires JSON.
- Strict schema validation afterward: known fields only, valid enums, bounded
  titles and item counts, parseable dates. Date-only values resolve in the
  user's timezone and are stored as UTC.
- The prompt receives the user's timezone and current local date/time so
  relative dates ("tomorrow", "December 15") resolve correctly.
- Items without a deadline get `due_at = null`; the model is instructed not to
  invent dates or reminders.
- User text is delimited inside a JSON envelope and treated as untrusted data,
  not instructions.
- Suggestions are returned to the client and are **not persisted** until the
  user reviews and saves them through `POST /api/items`.
- Rate limiting is per user and cache-backed. Input length, output size and item
  counts are bounded.
- Provider failures map to typed, secret-free HTTP errors (409 disabled or
  unconfigured, 422 invalid input, 429 rate limited, 504 timeout, 502 provider
  or invalid response). Manual capture still works.
- The server key and the per-user key both go through the same resolver. A
  personal key is sealed with AES-256-GCM and is never logged, returned or
  written to browser storage.
- Tests inject a mock `AiProvider` into `ctx.shared_store`; client tests use a
  local Axum mock server. CI needs no real API key.

## 9. Notifications

Telegram and Discord sit behind a small provider interface:

```text
NotificationSender
  send(channel, credentials, message) -> Result<(), NotificationError>
```

- Telegram: Bot API `sendMessage` with a bot token and chat id. Messages are
  plain text with no `parse_mode`, so user titles cannot inject formatting.
- Discord: webhook POST with `allowed_mentions.parse = []`, so item titles
  cannot ping users, roles or `@everyone`. No gateway connection is maintained.
- Both clients use a bounded request timeout, disable redirects, cap the
  response body and map status codes to typed retryable/permanent errors.
- Destinations are restricted to the approved provider endpoints. The Telegram
  API root must be `https://api.telegram.org`; Discord webhooks must be HTTPS
  on an official Discord host with exactly `/api/webhooks/<id>/<token>` and no
  port, credentials, query or fragment. This prevents SSRF.
- Credentials are sealed with AES-256-GCM (`v1:` envelope), never returned by
  the API and never logged. Provider errors are sanitized and truncated.
- `POST /api/notifications/test` sends one message through an enabled,
  configured channel and is rate-limited per user. It ignores the master switch
  so a user can verify a channel before enabling notifications.

Every intended delivery is persisted in `notification_deliveries` **before** a
`NotificationDeliveryWorker` job is enqueued. The job carries only the row id;
the worker re-reads preferences, credentials and item state, so edits,
completion, deletion or disabling a channel between enqueue and send are safe.
An atomic `pending → processing` update means only one worker can send a row,
and a row is only claimable once its retry backoff has expired, so a duplicate
job cannot bypass the wait.
Retryable failures go back to `pending` with exponential backoff, respecting
`Retry-After`; permanent failures end in `failed`. A delivery that is no longer
relevant ends in `cancelled`. The queue itself has retries disabled because the
database record owns retry state.

The unique `deduplication_key` is the real guard against duplicates: repeated
scheduler scans and restart recovery insert through it and only the first insert
wins. Keys are built from the item UUID + reminder timestamp + channel, the
user + local date + channel for digests, and the user + local date + channel for
overdue summaries. A reminder scan only considers open items whose `remind_at`
is inside the configured grace window; the worker rechecks the item status and
the exact reminder timestamp.

Scheduled scans (configured in `config/<env>.yaml` under `scheduler:` and run by
`cargo loco start --all` / `braindrop-cli start --all`):

| Schedule | Task | Purpose |
|---|---|---|
| every minute | `notifications:reminders` | enqueue due reminders |
| every minute | `notifications:digests` | enqueue daily digests at user-local time |
| hourly | `notifications:overdue` | enqueue grouped overdue summaries |
| every minute | `notifications:recover` | requeue pending and abandoned deliveries |
| daily at 03:15 | `notifications:cleanup` | delete finished history past retention |

The daily digest is eligible inside a bounded local window after the configured
time, with midnight-wrap handling that keeps a cross-midnight window on one
local day. Empty digests are skipped. Overdue summaries are grouped once per
user, channel and local day and become eligible at the user's daily delivery
time. History retention defaults to 30 days.

Loco runs the queue worker in-process; `queue.num_workers` bounds concurrency.
On boot, the notification initializer also runs recovery when the queue is in
`BackgroundQueue` mode, as a fast path for restarts. Tests install a recording
`NotificationSender` in `ctx.shared_store` and call the scan functions with a
deterministic clock; CI needs no real provider credentials.

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
- Light/dark/system theme. The account setting is applied on load and saved by
  the Settings page; the header toggle stays instant.
- Quick capture is a modal in the shell. Manual capture calls `POST /api/items`
  and always remains available. "Organize with AI" calls `/api/ai/parse`, then
  opens editable suggestion cards with selection, removal, retry and a
  "Save selected" action that posts only the chosen suggestions. AI errors keep
  the dialog in manual mode.
- Pages are lazy-loaded per route with `React.lazy` and `Suspense`, keeping the
  initial bundle focused on the shell and authentication.
- Item views share an `ItemWorkspace` (search, filters, sort, pagination,
  create/edit dialog, delete confirmation, complete/reopen). Today and Upcoming
  derive their windows from the account timezone.
- Settings covers timezone, theme, default type/priority and week start.
- Integrations covers optional OpenRouter settings: enable/disable AI, choose
  server or personal credentials, override the model, and replace/remove a
  personal key. The resolved status (available/disabled/not configured) is
  computed server-side. Keys are write-only and masked.
- TanStack Query invalidates item lists, statistics and settings after each
  mutation so counts and views stay consistent.
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
| `REMOTE_IP_SOURCE` | client-address source for auth rate limits: `ConnectInfo` (default) or `RightmostXForwardedFor` behind one trusted proxy | no |
| `ALLOW_REGISTRATION` | enable public registration | no (default false in milestone 2) |
| `REQUIRE_EMAIL_VERIFICATION` | require verified email to log in | no |
| `AI_ENABLED` | server-wide AI switch | no (default false) |
| `OPENROUTER_API_KEY` | optional server AI provider key | no |
| `OPENROUTER_MODEL` | optional default model id | no |
| `OPENROUTER_BASE_URL` | provider base URL (server-only) | no |
| `AI_TIMEOUT_SECONDS` | provider request timeout | no (default 30) |
| `AI_MAX_INPUT_CHARS` | capture text limit | no (default 8000) |
| `AI_RATE_LIMIT_PER_MINUTE` | per-user request limit | no (default 10) |
| `AI_ENCRYPTION_KEY` | base64 32-byte key for personal provider keys | required for BYOK |
| `NOTIFICATION_ENCRYPTION_KEY` | base64 32-byte key for Telegram/Discord credentials | recommended if notifications used; falls back to `AI_ENCRYPTION_KEY` |
| `NOTIFICATION_TIMEOUT_SECONDS` | provider request timeout | no (default 10) |
| `NOTIFICATION_MAX_ATTEMPTS` | attempts before a delivery is failed | no (default 5) |
| `NOTIFICATION_RETRY_BASE_SECONDS` | first retry delay | no (default 30) |
| `NOTIFICATION_RETRY_MAX_SECONDS` | maximum retry delay | no (default 3600) |
| `NOTIFICATION_HISTORY_RETENTION_DAYS` | finished delivery retention | no (default 30) |
| `NOTIFICATION_TEST_RATE_LIMIT` | per-user test sends per minute | no (default 5) |
| `MAILER_*` | SMTP settings | production |

Development config contains a generated JWT secret and local SQLite defaults;
those are not production secrets. Production config has no defaults for secrets
and refuses to boot without them.

## 13. Security

- Authenticated user ownership on every data query.
- 404 (not 403) for another user's resources.
- No secrets in API responses, logs or error messages; SQLite error strings from
  notification operations are redacted before they reach a response or a
  delivery record.
- Authenticated encryption for provider credentials.
- SSRF-resistant outbound destinations; the OpenRouter client also disables
  redirects because its base URL is operator configuration that may carry a key.
- Input limits on AI text, titles, tags and search; a 1 MiB request-body limit,
  and validation of registration, password-reset and reset-token fields.
- Auth endpoints are rate-limited per account/token and per client address.
  Counters live in Loco's in-process cache and reset on restart, so edge limits
  are recommended for public exposure. `REMOTE_IP_SOURCE` controls the client
  address: `ConnectInfo` (socket peer) by default, or `RightmostXForwardedFor`
  only behind one trusted proxy that overwrites `X-Forwarded-For`.
- Response headers are explicit in all environments: a strict CSP
  (`script-src 'self'`, `script-src-attr 'none'`, no inline scripts or
  `unsafe-eval`, `object-src 'none'`, `base-uri 'none'`, `frame-ancestors 'none'`,
  `form-action 'self'`; `'unsafe-inline'` is limited to styles), plus
  `X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY`,
  `Referrer-Policy: no-referrer`, `Permissions-Policy`,
  `Cross-Origin-Opener-Policy: same-origin` and
  `Cross-Origin-Resource-Policy: same-origin`. HSTS is deliberately **not** set
  by the app, which cannot see TLS; the reverse proxy must set it.
- `src/middleware/cache_control.rs` adds `Cache-Control: no-store` to `/api/*`,
  `/_*` and HTML documents. Hashed static assets stay cacheable, and the service
  worker bypasses API and monitoring paths.
- CSRF is not a concern for bearer-token API calls, but the generated frontend
  still stores the JWT in `localStorage`. The strict CSP and no-store policy are
  compensating controls, not a cure: any XSS is a potential token compromise.
- Dependencies are kept current; CI runs `cargo clippy` and frontend lint/tests.
  Third-party GitHub Actions are pinned to commit SHAs. See
  [`SECURITY.md`](../SECURITY.md) for the supported version and disclosure
  process.

## 14. Testing

Backend uses Loco's request/model testing conventions:

- Auth: registration, login, current user, verification.
- Registration/verification policy, including `admin:create` idempotency.
- Authorization and cross-user isolation for every item/settings endpoint.
- Validation and error shapes.
- AI parsing with a mocked provider: success, multiple suggestions, malformed
  and oversized responses, unsupported structured output, auth/rate-limit/
  timeout errors, disabled/unconfigured states, per-user key isolation, encrypted
  storage, timezone handling and the guarantee that suggestions are not
  persisted.
- Notification delivery with mocked providers.
- Scheduler deduplication and restart recovery.
- Disabled public registration.

Frontend:

- TypeScript type-check and production build.
- ESLint.
- Vitest for utilities/components, including editing, excluding and removing AI
  suggestions and converting only the selected ones into item payloads.
- Playwright smoke tests for protected-route redirect, registration visibility,
  login → dashboard → logout, bad-credential handling, item lifecycle, the PWA
  manifest, service-worker registration (including that no `/api/*` path is
  cached) and AI-assisted capture. The AI flow uses a local mock provider and
  covers both successful review/save and manual fallback after a provider error.

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

Milestone 6 added:

- `docker-compose.yml` for real deployments and `.env.example`.
- Persistent named volume for both SQLite files and their WAL/SHM sidecars.
- `start --all` in the image: HTTP, queue worker and scheduler in one process.
- Release profile tuning (`lto = "thin"`, `codegen-units = 1`, `strip`).
- Six native release targets, `scripts/build-native.sh`, `scripts/package.sh`,
  `dist/braindrop-<platform>/` plus `.tar.gz` and SHA-256 sidecars.
- A documented `Makefile` and `scripts/deploy.sh` that uses only `ssh`/`scp`,
  stages uploads, activates releases atomically and keeps `shared/.env` and data.
- Task 007 hardened that staging path: uploads use an unpredictable `mktemp`
  file under the SSH user's home with `umask 077`, extraction goes to a `0700`
  staging directory, and a cleanup trap removes both on success or failure.
- `scripts/deploy-init-alpine.sh` (OpenRC) and
  `scripts/deploy-init-ubuntu.sh` (systemd), both with an unprivileged user.
- `scripts/backup.sh` / `scripts/restore.sh` using SQLite's online backup API,
  with integrity checks, retention and an explicit restore confirmation.
- Deployment, persistence and backup checks inside `./scripts/ci.sh`.

The SPA is served from `frontend/dist` on disk. Loco 1.3.1's `embedded_assets`
only scans an `assets/` directory, so embedding would require moving the
generated frontend layout; the supported artifact ships `frontend/dist`.

Single replica for SQLite. Health checks use Loco's `/_health` and `/_readiness`.
Publishing to GHCR remains prepared but disabled; GitHub Actions is manual-only
and no release is published.

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
| **JWT in localStorage** | Generated Loco frontend behavior. Task 007 added a strict CSP, no-store private responses and an external pre-paint script; XSS-based token theft remains a documented residual risk, and cookie mode is still optional future work. |
| **Explicit headers, no HSTS** | Loco presets can clear browser storage or fight the PWA's static caching. The app sets a reviewed header set; HSTS belongs at the TLS-terminating proxy, which the app cannot see. |
| **In-process auth rate limits** | One replica by design, so a cache-backed fixed window is enough. Counters reset on restart; public deployments should also limit at the edge. |
| **Server key plus optional BYOK** | A server key keeps AI usable without user setup; a personal key lets users pay for their own usage. Both resolve through one typed path, and neither is ever returned by the API. |
| **AES-256-GCM for personal keys** | Authenticated encryption with an externally configured key. The envelope is versioned (`v1:`) so a future key rotation can be handled explicitly. |
| **Suggestions are never auto-saved** | The review step is the product contract: AI output is untrusted data, and the user chooses what becomes an item. |
| **In-memory cache for rate limits** | The deployment is single-instance. A restart clears counters, which is acceptable; a shared cache is unnecessary complexity today. |
| **Mock provider seam** | Tests and Playwright never call OpenRouter. The controller reads an optional provider from `shared_store`, and e2e uses a local HTTP mock. |

## 17. Open questions

- Exact FTS strategy for input search: FTS5 virtual table vs. indexed `LIKE`.
- Digest scheduling granularity and default send time.
- TierHive runtime capabilities and the exact SSH deployment contract.
