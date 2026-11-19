# BrainDrop Progress

Last updated: **2026-11-19**

## Milestones

| # | Milestone | Status | Notes |
|---|---|---|---|
| 001 | Bootstrap | **Complete** | Docs, scaffold verification, SQLite + SQLite queue, Mantine shell, builds and tests |
| 002 | Auth and shell hardening | **Complete** | Registration/verification config, admin task, PWA, Playwright smoke, Docker-first CI |
| 003 | Items and settings | **Complete** | Items CRUD, filtering, pagination, stats, per-user settings, item UI |
| 004 | AI capture | **Complete** | OpenRouter client, `/api/ai/parse`, encrypted BYOK, review UI, mocked tests |
| 005 | Notifications | **Complete** | Telegram/Discord clients, durable delivery queue, scheduler scans, settings UI, mocked tests |
| 006 | Deployment and distribution | **Complete** | Docker Compose, six native targets, Makefile + SSH release flow, backups/restore, deployment CI and measurements |
| 007 | Release | **Complete** | Security hardening, CSP/headers, release docs, security regression tests, v0.1.0 readiness — `READY WITH LIMITATIONS` |

All milestones are complete. Milestone 007 remains gated: nothing was pushed,
tagged, published or deployed without explicit authorization.

## Milestone 001 — Bootstrap

### Completed

- Inspected the existing repository before changing anything. A complete Loco
  scaffold was already present (SQLite, `queue: kind: Sqlite`, `--assets
  clientside`), so it was preserved instead of regenerated.
- Confirmed the toolchain:
  - `loco` 1.3.1, `loco-rs` 1.3.1
  - `sea-orm-cli` 2.0.4
  - Rust 1.100.0, Cargo 1.100.0
  - Node 24.14.1, pnpm 10.12.4 (via Corepack), npm 11.11.0
- Confirmed SQLite configuration in `config/{development,test,production}.yaml`:
  - app DB: `sqlite://braindrop_<env>.sqlite?mode=rwc`
  - queue: `kind: Sqlite`, `sqlite://loco_<env>.sqlite?mode=rwc`
  - Loco 1.3 enables WAL automatically for file-backed SQLite.
- Preserved generated authentication (`/api/auth/*`), users model, mailers,
  tasks, workers and tests.
- Added project documentation: `README.md`, `LICENSE`, `docs/SPEC.md`,
  `docs/ARCHITECTURE.md`, `docs/PROGRESS.md`, `docs/DEPLOYMENT.md`, and the
  seven milestone task files.
- Extended the React SPA with Mantine 9:
  - `MantineProvider`, Mantine core/dates/notifications styles
  - AppShell with responsive navbar, mobile burger, header, user menu
  - light/dark/system theme toggle
  - quick-capture modal placeholder
  - login and registration screens restyled
  - placeholder pages for Dashboard, Inbox, Today, Upcoming, All items,
    Completed, Settings and Integrations
  - TanStack Query current-user hook
  - shared empty/loading/error building blocks
- Added an explicit `/api` catch-all controller so unknown API routes return a
  JSON 404 instead of the SPA `index.html`.
- Added frontend tooling: ESLint, Vitest, type-check script, PostCSS Mantine
  preset, Vitest setup.
- Initialized a Git repository (the directory was not a repo) and committed the
  milestone as `chore: bootstrap BrainDrop on Loco with a Mantine shell`.

### Validation

Backend:

```text
cargo fmt --all -- --check        passed
cargo clippy --all-targets -- -D warnings
                                  passed
cargo test                        passed — 29 tests (2 unit, 27 request/model/task)
cargo loco routes                 auth + monitoring + /api fallback registered
```

Frontend:

```text
pnpm --dir frontend typecheck     passed
pnpm --dir frontend lint          passed
pnpm --dir frontend test          passed — 3 tests
pnpm --dir frontend build         passed — dist/index.html + hashed JS/CSS
```

Runtime smoke test (`PORT=5199 cargo loco start`):

```text
GET /_health              200
GET /                     200 text/html
GET /dashboard            200 text/html (SPA fallback)
GET /api                  404 application/json
GET /api/does-not-exist   404 application/json
GET /api/auth/does-not-exist 404 application/json
```

The frontend build reports a >500 kB chunk warning (Mantine + Tabler icons).
It is not a failure; route-level code splitting is deferred.

### Decisions

- **Keep Loco 1.3.1.** The delivered scaffold is complete and current. The
  specification names 1.2.x, but downgrading would discard a working generated
  app (SeaORM 2, React 19, React Router 8) and risks breaking generated auth and
  tests. The conventions are unchanged; the deviation is documented in
  `ARCHITECTURE.md`.
- **Do not regenerate.** `loco new` output is already present; re-running it
  would overwrite the repository. The generator compatibility was verified by
  inspecting the generated structure and installed tool versions.
- **Minimal shell only.** Item persistence, AI and notifications are deliberately
  absent so milestone 001 stays independently testable.
- **Commit without co-author** for milestone 001, per the initial execution
  instructions. Later implementation commits follow the repository rule and add
  the implementing agent (Codex or OpenCode) as co-author.

### Blockers

None.

### Known limitations

- The quick-capture modal is a shell placeholder; it does not persist items yet.
- Dashboard counts are placeholders (`—`) until `/api/stats` exists.
- Settings and Integrations pages are informational placeholders.
- Registration was still the generated open flow and there was no Docker, CI
  publishing, backup script or PWA yet; milestone 002 replaced registration with
  the configurable policy and added Docker-first CI and the PWA shell.
- `.github/workflows/ci.yaml` was the generated workflow and referenced
  Redis/Postgres and npm; milestone 002 replaced it with a manual-only workflow
  that calls `./scripts/ci.sh`.
- Development SMTP requires a local catcher (e.g. Mailpit) for verification mail.

## Milestone 002 — Auth and Shell Hardening

### Completed

- Added a typed authentication policy in `src/auth_policy.rs`
  (`AuthPolicy::from_config`), read from `settings.auth` through
  `Config::settings<T>()`. Application code never calls `std::env`.
  - `ALLOW_REGISTRATION` (default `false` in development/production).
  - `REQUIRE_EMAIL_VERIFICATION` (default `false`).
- Gated `POST /api/auth/register`: returns `403` with the machine-readable code
  `registration_disabled` when public registration is off.
- Enforced email verification on login when configured: returns `403`
  `email_verification_required` for unverified accounts.
- Added the public `GET /api/auth/config` route, which returns only the two
  policy booleans. It sits outside the ten business endpoints and needs no token.
- Added the native Loco task `admin:create` (`src/tasks/admin_create.rs`):
  - creates a verified account by default;
  - `verified:false` creates an unverified account and sends the verification
    mail;
  - `force:true` resets an existing account;
  - duplicate accounts fail without `force`, so repeated setup is explicit.
- Fixed a real deployment hazard: Loco base64-decodes `JWT_SECRET` before
  signing, and a signing failure was previously reported as `401`. Token
  signing now lives in `issue_token` and returns `500` with a clear log message,
  and `config/production.yaml` documents that the secret must be base64.
- Hardened the SPA shell:
  - `RequireAuth` validates the stored token through `/api/auth/current`, shows
    `FullPageLoader` while checking, and clears token + query cache on failure.
  - Login and Register read the public auth config; the register link and route
    are hidden when registration is disabled.
  - `api/client.ts` parses both the Loco error body and the shared `ApiError`
    shape; logout clears client state.
  - Accessible skip link, focus styles and shell copy updates.
- Added PWA installability:
  - `frontend/public/manifest.webmanifest`, `favicon.svg` and generated PNG
    icons.
  - `frontend/public/sw.js` caches only the app shell and static assets;
    `/api/*`, `/_*` and all non-GET requests always go to the network.
  - Registered in production builds only (`frontend/src/pwa.ts`), never in
    development.
- Added Playwright smoke coverage (`frontend/playwright.config.ts`,
  `frontend/e2e/auth.spec.ts`, `frontend/e2e/pwa.spec.ts`): protected-route
  redirect, registration hidden when disabled, login → dashboard → logout,
  bad-credential handling without a full reload, manifest delivery and
  service-worker registration (asserting no `/api/*` path is cached).
- Added Docker-first local CI:
  - production multi-stage `Dockerfile` (pnpm frontend build, Rust release
    build, slim non-root runtime, health check) and `.dockerignore`;
  - `docker/ci/{rust,frontend,playwright}.Dockerfile` and
    `docker-compose.ci.yml`;
  - `scripts/ci.sh` as the single entry point, plus `scripts/ci-rust.sh` and
    `scripts/e2e-server.sh`;
  - Cargo caches in named Docker volumes and the pnpm store in a BuildKit cache
    mount; test databases live inside
    containers, never in the repository;
  - Playwright runs in the app container's network namespace over
    `127.0.0.1:5150`, because Chromium upgrades non-localhost HTTP origins to
    HTTPS.
- Replaced `.github/workflows/ci.yaml` with a manual-only
  (`workflow_dispatch`) workflow that calls `./scripts/ci.sh`, so hosted and
  local runs cannot drift. Automatic runs are intentionally disabled for now.
- Updated `AGENTS.md`, `README.md`, `docs/SPEC.md`, `docs/ARCHITECTURE.md`,
  `docs/DEPLOYMENT.md` and `docs/tasks/002-auth-and-shell.md` with the new auth
  policy and CI contract.

### Validation

Single command, Docker-first:

```text
./scripts/ci.sh                   passed end to end
```

Contents:

```text
Rust fmt / clippy / tests         passed — 39 tests (4 unit, 35 integration)
SQLite migrations                 passed — throwaway database
Frontend typecheck / lint / tests passed — 3 Vitest tests
Frontend production build         passed
Production image + /_health       passed
Playwright smoke tests            passed — 6 tests
```

### Decisions

- **Policy on `settings.auth`.** Loco already deserializes typed settings;
  a small typed struct is less error-prone than reading keys ad hoc and keeps
  the flags testable through `ctx.config`.
- **Registration disabled by default.** The specification requires it; tests
  enable registration explicitly so the generated auth flow stays covered.
- **New `admin:create` task instead of changing `user:create`.** The generated
  task keeps its meaning; the new task carries verification and `force`
  semantics and is the documented way to create the first account.
- **Playwright shares the app's network namespace.** It exercises the real
  production image on `127.0.0.1`, which also avoids Chromium's HTTPS upgrade
  for non-localhost origins.
- **CI builds the production image now.** Milestone 6 still owns full deployment
  Compose, publishing, backups and the TierHive template, but validating the
  image early prevents an unbuildable deployment from accumulating.

### Blockers

None.

### Known limitations

- Quick capture remains a shell placeholder; it does not persist items until
  milestone 3.
- Dashboard counts and Settings/Integrations pages are still placeholders.
- `docker-compose.ci.yml` is a CI harness, not the deployment compose file.
  Publishing, backups and the TierHive template remain milestone 6 work.
- GitHub Actions is manual-only by instruction; it has not been run on GitHub in
  this milestone.
- The frontend build still reports a >500 kB chunk warning (Mantine + Tabler
  icons); route-level code splitting is deferred.
- No strict Content-Security-Policy yet. The JWT stays in `localStorage`, as
  generated by Loco; CSP/XSS hardening remains in milestone 7.
- Development SMTP requires a local catcher (e.g. Mailpit) for verification and
  reset mail.

## Milestone 003 — Items and Settings

### Completed

- Generated the `items` and `user_settings` models and migrations with the
  native Loco generators, then extended the migrations by hand:
  - `items.pid` (UUID, unique) is the public identifier; `user_id` is an
    indexed foreign key with `ON DELETE CASCADE`.
  - Indexes for `(user_id, status)`, `(user_id, due_at)`,
    `(user_id, remind_at)`, `(user_id, item_type)` and
    `(user_id, created_at)`.
  - The database column is `item_type` (avoiding the Rust keyword) while the
    JSON field is `type`.
  - `user_settings.user_id` is unique and cascades with the user.
- Implemented the typed domain model in `src/models/items.rs`:
  - enums for type, status and priority with database/JSON round-tripping;
  - tag normalization (trim, drop empties, case-insensitive de-duplication);
  - scoped finders, list/filter/sort/pagination and text search;
  - transactional `create_many` for one-or-many capture;
  - complete/reopen/archive/update/delete transitions that set their
    timestamps.
- Implemented `src/models/user_settings.rs` (defaults, `get_or_create`,
  scoped update) and `src/models/stats.rs` (dashboard counts).
- Added timezone helpers in `src/time.rs` using `chrono-tz`. Stats and the
  Today/Upcoming views compute day/week windows in the user's timezone and
  compare them as UTC.
- Added request/response DTOs with `ts-rs` exports:
  `src/dtos/items.rs` and `src/dtos/settings.rs`.
  - `PATCH` uses a `Patch<T>` newtype so an absent field and an explicit
    `null` are distinguishable.
  - `PATCH /api/settings` follows the same partial-update rules.
- Implemented the controllers and wired the routes in `src/app.rs`:
  - `GET/POST /api/items`, `GET/PATCH/DELETE /api/items/:id`
  - `GET /api/stats`, `GET/PATCH /api/settings`
  - every lookup is scoped by `user_id`; another user's id (or an
    unparseable UUID) returns 404.
  - batch creation returns 201 with `{ "items": [...] }`; an invalid item
    rolls the whole batch back.
  - invalid enum values in a JSON body return 422; unknown list statuses
    return 422.
- Built the SPA item experience:
  - real Dashboard (stat cards, due-today and recent-inbox previews), Inbox,
    Today, Upcoming, All items, Completed and Settings pages;
  - shared `ItemWorkspace` with create/edit dialog, delete confirmation,
    complete/reopen, search, filters, sort and pagination;
  - quick capture in the shell now calls `POST /api/items`;
  - Settings edits timezone (IANA list), theme, default type/priority and week
    start, and the stored theme is applied on load;
  - route-level lazy loading via `React.lazy` and `Suspense`;
  - TanStack Query invalidation keeps item lists, stats and settings fresh.
- Tests added:
  - Rust unit tests for timezone helpers and item model rules;
  - request tests for CRUD, batch atomicity, cross-user isolation, invalid
    input, search/filter/pagination, stats, timezone day boundaries and
    settings persistence;
  - Vitest tests for item helpers, query building and `ItemCard` interactions;
  - Playwright `items.spec.ts` covering capture, list, complete/reopen, edit
    and delete.

### Validation

Single command, Docker-first:

```text
./scripts/ci.sh                   passed end to end
```

Contents:

```text
Rust fmt / clippy / tests         passed — 78 tests (23 unit, 55 integration)
SQLite migrations                 passed — throwaway database
Frontend typecheck / lint / tests passed — 22 Vitest tests
Frontend production build         passed
Production image + /_health       passed
Playwright smoke tests            passed — 8 tests
```

### Decisions

- **Public identifiers are UUID `pid` values.** Integer primary keys never
  leave the API, and every lookup still includes the authenticated user.
- **One batch endpoint.** `POST /api/items` accepts a single object or
  `{ "items": [...] }`; inserts run in one transaction so a partial capture is
  impossible.
- **`Patch<T>` instead of sentinel values.** It makes "leave unchanged" and
  "clear this field" explicit without hand-written deserializers.
- **`item_type` in SQLite, `type` in JSON.** The column name avoids the Rust
  keyword while the API keeps the vocabulary from the specification.
- **Timezone-aware statistics.** The user's IANA timezone (default `UTC`)
  defines "today" and "this week"; storage and comparisons stay in UTC.
- **Priority sorting uses a SQL rank.** `urgent → high → normal → low` is
  ordered correctly instead of alphabetically.
- **422 for invalid enums.** Enum values are validated in the DTO; an unknown
  status in a list filter also returns 422 rather than silently returning an
  empty page.
- **No FTS5 yet.** Indexed `LIKE` search meets the acceptance criteria at this
  scale; FTS can be added later without changing the API.

### Blockers

None.

### Known limitations

- Search is `LIKE`-based; no FTS5 ranking or stemming yet.
- AI capture, notification preferences and delivery remain milestones 4–5. The
  Integrations page is still a placeholder.
- `docker-compose.ci.yml` is a CI harness, not the deployment compose file.
- GitHub Actions is manual-only by instruction.
- The main frontend chunk is still above 500 kB (Mantine + Tabler icons) even
  though pages are lazy-loaded; the warning is non-fatal.
- No strict Content-Security-Policy yet; CSP/XSS hardening remains milestone 7.

## Milestone 004 — AI Capture

### Completed

- Added the `reqwest` (rustls, no OpenSSL), `aes-gcm` and `base64`
  dependencies, plus the Tokio `net`/`time` features used by the client and the
  timeout wrapper.
- Added migration `m20261118_161319_add_ai_settings`:
  - `user_settings.ai_enabled`
  - `user_settings.ai_use_server_key`
  - `user_settings.ai_model`
  - `user_settings.ai_api_key_encrypted`
  - a working `down()` that drops the columns in reverse order.
- Added typed `settings.ai` configuration in all three environments with safe
  defaults:
  - `AI_ENABLED` (false), `OPENROUTER_API_KEY`, `OPENROUTER_MODEL`,
    `OPENROUTER_BASE_URL`, `AI_TIMEOUT_SECONDS`, `AI_MAX_INPUT_CHARS`,
    `AI_RATE_LIMIT_PER_MINUTE`, `AI_ENCRYPTION_KEY`.
  - An unset YAML value is treated as an empty string, so an installation with
    no AI credentials still boots.
  - Added `cache: kind: InMem` for per-user rate limiting.
- Implemented `src/ai/`:
  - `client.rs`: an `AiProvider` trait and an OpenRouter chat-completions client
    with a connect/request timeout, bounded response body and status mapping.
    If a model rejects `response_format`, the client retries once without it.
  - `prompt.rs`: a delimited JSON envelope for untrusted capture text, the
    user's timezone and local date/time, and an explicit JSON schema.
  - `suggestions.rs`: strict validation of every suggestion (known fields,
    enums, title bounds, item limit, dates); date-only values resolve in the
    user's timezone and are stored as UTC.
  - `credentials.rs`: AES-256-GCM `SecretBox` with the versioned envelope
    `v1:<base64url(nonce || ciphertext || tag)>`.
  - `availability.rs`: one resolver for the server key and personal keys, with
    a safe status and reason for the UI.
  - `rate_limit.rs`: a cache-backed per-user limit that fails open if the cache
    is unavailable.
- Added `POST /api/ai/parse` (JWT auth):
  - validates and trims input before any provider call;
  - resolves credentials and the rate limit before spending a provider request;
  - never persists suggestions;
  - logs metadata only (user id, item count, credential source);
  - maps failures to 409 (disabled/unconfigured), 422 (invalid input), 429
    (rate limit), 504 (timeout) and 502 (auth/provider/invalid/oversized).
  - the controller reads an optional `AiProvider` from `ctx.shared_store` so
    tests can inject a mock; otherwise it builds the real client.
- Extended settings:
  - `SettingsPatch` supports `ai_enabled`, `ai_use_server_key`, `ai_model` and
    an encrypted-key set/clear operation.
  - `UpdateSettingsParams` accepts a write-only `ai_api_key`; an empty string
    removes the stored key, a non-empty value replaces it.
  - `SettingsResponse` now includes an `AiSettingsResponse` with status,
    reason, active model, credential source, `has_personal_key` and the server
    default model. The key itself is never serialized.
- Built the frontend:
  - quick capture keeps manual save and adds "Organize with AI", a processing
    indicator, retry/error handling and an editable suggestion review with
    selection, removal and "Save selected";
  - the manual path stays usable when AI is disabled, unconfigured or failing;
  - the Integrations page replaces the OpenRouter placeholder with enable/
    disable, server-vs-personal credential choice, model override and
    replace/remove personal key controls, plus the resolved AI status;
  - personal keys are masked, write-only and never stored in browser storage;
  - the Settings note now points to Integrations.
- Added tests:
  - Rust unit tests for config bounds, the encryption envelope, suggestions,
    availability and the client (using a local Axum mock server for status,
    retry and timeout behavior);
  - request tests with a mock provider for success, multiple suggestions,
    malformed output, auth failures, per-user rate limiting, timeout, account
    and server disablement, missing credentials, timezone/relative dates, no
    persistence, encrypted storage, per-user key isolation and non-disclosure;
  - Vitest tests for suggestion conversion, editing/excluding/removing and
    error messages;
  - Playwright AI capture against a local mock provider: review/edit/remove/
    save and manual fallback after a provider error. CI needs no real key.

### Validation

Single command, Docker-first:

```text
./scripts/ci.sh                   passed end to end
```

Contents:

```text
Rust fmt / clippy / tests         passed — 131 tests (64 unit, 67 integration)
SQLite migrations                 passed — throwaway database
Frontend typecheck / lint / tests passed — 29 Vitest tests
Frontend production build         passed
Production image + /_health       passed
Playwright smoke tests            passed — 10 tests
```

### Decisions

- **Server key plus optional BYOK.** The server key keeps AI usable without
  setup; a personal key lets users pay for their own usage. Both resolve through
  one typed path.
- **Personal keys use AES-256-GCM.** The envelope is versioned so a future
  rotation can be explicit. `AI_ENCRYPTION_KEY` is required only for BYOK; the
  server-key path works without it.
- **Suggestions are never persisted.** The review step is the contract. AI
  output is untrusted data and the user chooses what becomes an item.
- **One structured-output retry.** Some chat-completions models reject
  `response_format`; the prompt still demands JSON, so one retry without the
  field keeps the feature usable without a model allowlist.
- **In-memory rate limiting.** The deployment is single-instance; a restart
  clearing counters is acceptable. A shared cache is unnecessary today.
- **Provider seam in `shared_store`.** Tests and Playwright never call
  OpenRouter. The controller uses the injected provider when present, which
  keeps request tests deterministic.
- **Safe AI defaults.** AI is off in production unless explicitly enabled, and
  every optional value has an empty default so the app boots without AI.

### Blockers

None.

### Known limitations

- Rate-limit counters are in memory and reset on restart.
- There is no streaming, tool calling, embeddings, semantic search or RAG.
- Personal keys require `AI_ENCRYPTION_KEY`; rotating it makes existing keys
  unreadable and users must re-enter them.
- The model field is free text passed to OpenRouter; there is no server-side
  model allowlist.
- Notification preferences and delivery were added in milestone 005.

## Milestone 005 — Notifications

### Completed

- Added migrations `m20261119_000001_add_notification_settings` and
  `m20261119_000002_notification_deliveries`:
  - ten `user_settings` columns for the master switch, channel switches and
    write-only credentials, reminder/digest/overdue preferences and the local
    digest time;
  - `notification_deliveries` with a unique `deduplication_key`, status,
    attempts, next-attempt/sent timestamps, sanitized `last_error`, user cascade
    and item `SET NULL`, plus indexes for due/processing/history queries.
- Added `src/models/notification_deliveries.rs` with typed channel, type and
  status enums, key builders, insert-on-conflict deduplication, atomic claim,
  sent/retry/failed/cancelled transitions, stale-processing reset and history
  cleanup.
- Added `src/notifications/`:
  - typed `settings.notifications` config with clamped limits and the
    encryption-key fallback;
  - AES-256-GCM credential sealing through the existing `SecretBox`;
  - strict URL/token/chat validation and official-host allowlists;
  - plain-text message rendering with length caps and mention suppression;
  - Telegram and Discord clients with timeouts, no redirects, capped response
    bodies and typed retryable/permanent error mapping;
  - dispatch/prepare logic that re-reads item state, preferences and credentials
    at send time;
  - reminder, digest, overdue, recovery and cleanup scans with deterministic
    clock entry points.
- Added `NotificationDeliveryWorker`, five native Loco tasks and the
  `Notifications` initializer. The worker owns retry state in the database, so
  the queue retry count is zero.
- Added scheduler entries in all environments: reminders, digests and recovery
  every minute; overdue hourly; cleanup daily at 03:15.
- Added `POST /api/notifications/test` with per-user rate limiting, typed
  conflict/validation/rate-limit/provider errors and no credential echo.
- Extended `GET/PATCH /api/settings` with notification preferences and channel
  status. Credentials are write-only: non-empty replaces, empty removes and
  omitted leaves unchanged. Responses expose only booleans, `configured` and the
  non-secret Telegram chat id.
- Built the frontend:
  - independent Telegram and Discord cards on Integrations with status, enable
    switch, write-only inputs, save, remove credentials, send test and brief
    setup instructions;
  - a Notifications section on Settings for the master switch, reminders, daily
    digest, local delivery time and overdue summaries;
  - generated TypeScript bindings and TanStack Query hooks for the new DTOs;
  - Vitest coverage for both pages and a Playwright configuration workflow.
- Updated the production/CI runtime to `start --all` and added a CI log assertion
  that Loco started the HTTP server, worker and scheduler.
- Added tests with a recording `NotificationSender` injected through
  `ctx.shared_store`; no CI run needs Telegram, Discord or AI credentials.
  Coverage includes both channels, disabled/missing configuration, sealed and
  non-disclosed secrets, cross-user isolation, rate limits, reminder dedup and
  cancellation after edit/delete/completion, digest timezone windows (including
  midnight wrap and DST helpers), overdue daily dedup, concurrent scans,
  retry/backoff, `Retry-After`, permanent failures, restart recovery and history
  cleanup.

### Validation

Single command, Docker-first:

```text
./scripts/ci.sh                   passed end to end
```

Contents:

```text
Rust fmt / clippy / tests         passed — 185 tests (95 unit, 90 integration)
SQLite migrations                 passed — throwaway database
Frontend typecheck / lint / tests passed — 36 Vitest tests
Frontend production build         passed
Production image + /_health       passed
Scheduler listing                 passed — five notification jobs
Worker and scheduler boot check   passed — start --all banner
Playwright smoke tests            passed — 11 tests
```

### Decisions

- **Durable row before queue job.** `notification_deliveries` is the source of
  truth. The queue only wakes a worker by id, so a crash between insert and
  enqueue is recoverable.
- **Deduplication in SQLite, not the queue.** Loco's `enqueue_unique` only
  deduplicates while a job is queued. The unique delivery key is durable across
  scans, retries and restarts.
- **Retries belong to the delivery record.** The queue's own retry is disabled;
  backoff, `Retry-After`, attempt limits and sanitized errors are stored on the
  row.
- **At-least-once, never exactly-once.** An ambiguous provider failure can be
  retried after the provider accepted the message. Documentation states this
  explicitly.
- **Official hosts only.** Telegram's base is fixed to the official HTTPS API;
  Discord webhooks are validated against official hosts and the exact path, with
  redirects disabled. There is no arbitrary webhook feature.
- **Plain text and no mentions.** Telegram gets no `parse_mode`; Discord gets an
  empty `allowed_mentions.parse`. User content cannot inject formatting or pings.
- **One key for both integrations.** `NOTIFICATION_ENCRYPTION_KEY` falls back to
  `AI_ENCRYPTION_KEY`, so a small deployment can keep one stable secret.
- **Test sends ignore the master switch.** The channel itself must be enabled
  and configured; this lets a user verify setup before turning notifications on.
- **Overdue summaries share the digest time.** They become eligible at the
  user's local daily delivery time, then the hourly scan sends the grouped
  summary once per local day.

### Blockers

None.

### Known limitations

- External delivery is at-least-once. A retry after an ambiguous failure can
  produce a duplicate message.
- The scheduler and worker run in one process for the single-replica
  deployment. Split processes are supported but not coordinated across replicas;
  SQLite remains single-writer.
- Notification history is stored in the application database and cleaned daily;
  the Loco queue's finished jobs use Loco defaults.
- There is no incoming Telegram/Discord message handling, bot command surface,
  gateway connection, email or browser push.
- Rate-limit counters for test sends are in memory and reset on restart.
- Digest and overdue scans run every minute/hour rather than at exact per-user
  times; the delivery window and local-day keys make repeated runs safe.

## Milestone 006 — Deployment and distribution

### Delivered

**Docker and Compose (006A)**

- Production `docker-compose.yml`, separate from the test-only
  `docker-compose.ci.yml`.
- Named volume `braindrop-data` at `/data` for the application database, the
  queue database and their WAL/SHM sidecars.
- `env_file: .env`, identical host/container port, health check, restart policy,
  `init: true` and a 30-second stop grace period.
- `Dockerfile` runs `start --all` and includes `sqlite3`, `tar`, `gzip` and
  `findutils` so backups run inside the container.
- Release profile: `opt-level = 3`, `lto = "thin"`, `codegen-units = 1`,
  `strip = "symbols"`, `panic = "unwind"`.
- `.env.example` with placeholders only. Mail is disabled by default, so a
  manual-only install boots without SMTP.
- Mailer config quotes optional strings so an empty `MAILER_HOST`, `MAILER_USER`
  or `MAILER_PASSWORD` is not parsed as YAML null.

**Native builds and distribution (006B)**

- Six targets: musl and glibc Linux (AMD64/ARM64), macOS (Apple Silicon/Intel).
- `scripts/build-native.sh`: musl through `messense/rust-musl-cross`, glibc
  through a `rust:1-bullseye` toolchain (glibc 2.31 baseline), macOS natively.
- `scripts/package.sh`: `dist/braindrop-<platform>/` with the binary, config,
  `frontend/dist`, scripts, `.env.example`, docs, `RELEASE`, plus a `.tar.gz`
  and SHA-256 sidecar. Layout checks reject databases and real `.env` files.
- `scripts/build-frontend.sh` uses the pinned pnpm version.

**Makefile and SSH deployment (006C)**

- Documented `Makefile` with the required variables and every workflow.
- `scripts/deploy.sh` uses only `ssh` and `scp`. It validates arguments, stages
  uploads, extracts atomically, preserves `shared/.env` and data, migrates,
  repoints `current`, restarts, health-checks, prunes old releases and rolls back
  the release (not the database) on failure.
- `scripts/deploy-init-alpine.sh` (OpenRC `supervise-daemon`, dedicated user,
  `need net`, logs) and `scripts/deploy-init-ubuntu.sh` (systemd `Type=simple`,
  `Restart=on-failure`, hardening, journald).
- Host-key verification is never disabled and the local `.env` is never
  uploaded. `init` generates the three encryption/JWT secrets once.

**Backups and restore (006D)**

- `scripts/backup.sh` uses SQLite's online backup API for both databases,
  integrity-checks each copy, writes a timestamped archive with manifest and
  checksum, and applies retention.
- `scripts/restore.sh` verifies checksum and integrity, requires explicit
  confirmation, moves current databases aside, restores both, fixes ownership,
  optionally migrates and restarts, and optionally polls health.
- `scripts/test-backup-restore.sh` proves the flow with real items, a settings
  row and a pending queue job.

**Validation and documentation (006E)**

- `./scripts/ci.sh` now also runs the backup/restore self-test, deploy argument
  validation, service-script syntax checks, package layout checks, a
  create-item/restart/persist probe and an in-container backup.
- Manual-only `release.yaml` prepared for native archives and a multi-arch GHCR
  image. It is not triggered and pushes only when `publish_image` is explicitly
  enabled.
- `README.md`, `AGENTS.md`, `docs/SPEC.md`, `docs/ARCHITECTURE.md`,
  `docs/DEPLOYMENT.md` and `docs/tasks/006-deployment.md` updated.

### Decisions

- **SPA stays on disk.** Loco 1.3.1's `embedded_assets` only discovers an
  `assets/` directory, so embedding `frontend/dist` would require moving the
  generated frontend layout. Every artifact ships `frontend/dist` and Loco's
  static middleware serves it.
- **glibc 2.31 baseline.** glibc binaries are built on Debian bullseye, not the
  newest distribution, so they run on Ubuntu 20.04 and newer. This is documented
  rather than discovered at runtime.
- **One process.** `start --all` runs HTTP, worker and scheduler together. Split
  processes remain supported but are not coordinated across replicas.
- **Release symlink, not in-place edits.** Deploys never overwrite a running
  release. `current` is repointed atomically and old releases stay for rollback.
- **No automatic database downgrade.** A failed deploy rolls back the binary and
  reports that a backup restore is required for schema rollback.
- **Secrets are created once.** `init` generates `JWT_SECRET`,
  `AI_ENCRYPTION_KEY` and `NOTIFICATION_ENCRYPTION_KEY` only when
  `shared/.env` does not exist. Rotation is a deliberate operator action.
- **CI stays Docker-first.** The new checks run in the same containers as the
  rest of the pipeline; no host toolchain is assumed.

### Validation

`./scripts/ci.sh` passes end to end. It covers:

```text
cargo fmt --check, clippy -D warnings, 185 Rust tests
clean SQLite migration
backup/restore self-test with real rows and a queue job
deploy argument validation and service-script syntax
distribution package layout check
frontend typecheck, lint, 36 Vitest tests, production build
production image health, worker/scheduler banner
create item -> restart container -> item still present
in-container SQLite backup
11 Playwright smoke tests
```

### Measurements

Taken on macOS 26.3.1, Apple M2, 16 GiB RAM, Docker 29.2.1, Rust 1.100.0 on
the host (1.98 in the Linux toolchain images) and SQLite 3.51.0. Sizes are the
bytes reported by `wc -c` and `docker images`; latency is a 50-request `curl`
average against a locally running production process.

**Release binaries and archives** (all six built successfully; SHA-256 sidecars
are written next to each archive):

| Target | Binary | Archive | SHA-256 (first 12) |
|---|---:|---:|---|
| `aarch64-apple-darwin` | 17,866,768 | 8,024,060 | `2948857cd102` (rebuilt in Task 007) |
| `x86_64-apple-darwin` | 19,012,152 | 8,514,514 | `a2d8311a10b1` |
| `aarch64-unknown-linux-musl` | 17,799,048 | 8,257,829 | `c58cf843ddaf` |
| `aarch64-unknown-linux-gnu` | 19,028,496 | 8,298,059 | `c5063699a6d1` |
| `x86_64-unknown-linux-musl` | 21,460,880 | 8,984,545 | `4cb4b37478c3` |
| `x86_64-unknown-linux-gnu` | 21,145,712 | 8,770,008 | `3fad5c1f599c` |

The Linux binaries are 17–20 MiB and the archives 7.6–8.6 MiB; each archive
contains the binary, config, built SPA, operational scripts and docs.

**Docker image** (`linux/arm64`, `braindrop-ci-app:latest`): ~190 MB on disk,
~42.8 MB as `docker save | gzip -1`, with a 19,110,512-byte stripped binary.
The image contains no compiler, Node.js or pnpm and runs as uid 10001.

**Startup and idle memory** (`start --all`, worker and scheduler confirmed in
every run):

| Runtime | Limit | Startup to `/_health` | Idle RSS | `/_health` avg | `/` avg |
|---|---:|---:|---:|---:|---:|
| macOS ARM64 package, host | — | 0.72 s | 27,792 KB | 0.41 ms | 0.48 ms |
| Docker image, arm64 | 128 MiB | 1.93 s | 13.72 MiB | 14.26 ms | — |
| Docker image, arm64 | 256 MiB | 1.91 s | 12.39 MiB | 2.09 ms | — |
| musl ARM64 package on Alpine 3.20 | 128 MiB | 0.26 s | 22.09 MiB | 2.08 ms | 3.12 ms |
| musl ARM64 package on Alpine 3.20 | 256 MiB | 0.23 s | 21.95 MiB | 2.08 ms | 3.12 ms |

The two Docker image latency figures were captured while cross-compilation was
running on the same host; the 14.26 ms value reflects that contention. Every
run boots the server, worker and scheduler, serves the SPA root and a deep link
(`/dashboard`), and passes migrations. Both 128 MiB and 256 MiB limits are
comfortable: the process settles at roughly 12–22 MiB RSS. A 128 MiB limit is
therefore sufficient for an idle single-replica install, with headroom for
request spikes; it is not a guarantee under sustained concurrent load.

**Executed here** (production config, temporary databases, no external
services): the macOS ARM64 package on the host, the `linux/arm64` Docker image,
and the musl ARM64 package inside `alpine:3.20`. **Cross-compiled but not
executed here**: macOS AMD64 (no Rosetta), both Linux AMD64 binaries (no amd64
runtime on this host) and the glibc ARM64 binary. Those paths are built,
packaged, checksummed and layout-checked, but only the three runtimes above
have been booted.

### Blockers

None.

### Known limitations

- No SSH deployment was executed against a real host, and no Alpine/Ubuntu
  service was booted on a real machine. Those paths are implemented, validated
  for syntax and arguments, and documented, but require an authorized target.
- AMD64 artefacts were cross-compiled on an Apple Silicon host, so their Linux
  toolchains ran under QEMU. They are correct but slower to produce (about
  2–5 minutes per target) than the native ARM64 and macOS builds.
- TierHive platform capabilities were not probed. The flow assumes SSH, `sh`,
  `tar`, `curl`, `sqlite3`/`sqlite` and `openssl`; `deploy-init` reports which
  service manager it finds.
- Publishing to GHCR and creating a GitHub release are prepared but disabled.
- Single replica only; SQLite remains single-writer.
- The queue database is recoverable but not transactional with the application
  database. A backup captures them sequentially.

## Milestone 007 — Release

### Delivered

**Security review and hardening**

- Independent read-only audit of auth, authorization, API, AI, notifications
  and deployment found no critical issues. Two high-severity rate-limit
  correctness gaps and several medium/low findings were fixed and re-tested.
- Authentication rate limits (`src/auth_rate_limit.rs`) use Loco's in-process
  cache with a process-wide mutex, so check-and-increment is atomic in the
  supported single-replica deployment. Login counts only failures, uses an
  account+client key, and is cleared on success: a known email cannot lock an
  account for other clients. Token endpoints keep one recovery attempt.
  Identity keys are bounded, and `REMOTE_IP_SOURCE` selects the client address.
- Registration and verification: duplicate registration now returns the same
  neutral success as a new registration (no account oracle); magic links honour
  the email-verification policy and consume the one-time token even when it is
  rejected; verification tokens expire after 24 hours; unknown-email logins
  perform a dummy hash for timing parity; raw database errors are no longer
  returned to clients.
- Input limits and redaction: 1 MiB payload limit, typed `RegisterParams` and
  `ResetParams` validation, no email addresses in auth logs, redacted
  notification provider errors, disabled OpenRouter redirects, and removal of
  the magic-link domain allowlist and its `regex` dependency.

**Transport and browser controls**

- Explicit `secure_headers` in development, test and production with `preset:
  empty`: strict CSP (no inline or eval scripts, `script-src-attr 'none',
  object-src 'none', base-uri 'none', frame-ancestors 'none'`),
  `X-Content-Type-Options`, `Referrer-Policy`, `X-Frame-Options` and
  `Content-Security-Policy`. `style-src 'unsafe-inline'` is retained because
  Mantine injects runtime styles; the residual CSS-injection surface is
  documented. HSTS is intentionally set at the TLS reverse proxy, not the app.
- New `CacheControl` middleware adds `Cache-Control: no-store` to `/api/*`,
  `/_*` and HTML responses while leaving hashed `/assets/*` cacheable; the
  service worker still refuses to cache API or private responses.
- The SPA CSP remains compatible: Vite vendor/route splitting removes the
  >500 kB warning, the pre-paint theme script is external, and Mantine runtime
  styles and PWA assets work under the policy. Accessibility/contrast tweaks
  cover dimmed text, button types, dialogs and sort labels.

**Release preparation**

- Open-source docs: `README.md` (real application screenshots and
  public-exposure guidance), `LICENSE`, `CHANGELOG.md`, `CONTRIBUTING.md`,
  `CODE_OF_CONDUCT.md`, `SECURITY.md`, `.env.example`,
  `docs/releases/v0.1.0.md`, and updates to `ARCHITECTURE.md` and
  `DEPLOYMENT.md`.
- GitHub Actions uses commit-pinned third-party actions and stays manual-only
  (`workflow_dispatch`); the release workflow pushes an image only when
  `publish_image=true`.
- SSH deploy stages uploads under a private `mktemp` directory with `umask
  077`, validates the path, and cleans it on exit. No fixed `/tmp/braindrop-*`
  upload path remains.
- Rust and frontend dependency audits are clean of exploitable production
  advisories; the remaining Rust `cargo audit --deny warnings` entries are
  unmaintained-crate warnings (`chrono`, `chrono-tz`, `unsafe-libyaml`) with no
  fixed upgrade, and no frontend advisories at moderate or above.

**Regression coverage**

- Rust request tests cover security headers/no-store, auth rate limits,
  verification-token expiry, magic-link policy, duplicate registration,
  registration validation, domain-allowlist removal and JWT query rejection.
- Playwright adds CSP/header/no-store checks and a full authenticated flow with
  a `securitypolicyviolation` listener; PWA cache hygiene is asserted. Real
  screenshots are generated only with `CAPTURE_SCREENSHOTS=1`.

### Validation

Single command, Docker-first:

```text
./scripts/ci.sh                   passed end to end (final run)
```

Contents and final counts from the passing run:

```text
Rust fmt / clippy -D warnings / migrations             passed
Rust unit tests                                       100 passed
Rust integration (requests/models) tests              98 passed
frontend typecheck / lint / production build           passed
frontend unit tests                                   36 passed (8 files)
production image health + worker/scheduler banner      passed
deployment persistence, static assets, in-container backup  passed
backup/restore self-test, deploy validation, package layout  passed
Playwright (smoke, AI, items, notifications, PWA, security)  14 passed, 1 skipped
```

The skipped Playwright case is the screenshot-capture spec, which runs only
when `CAPTURE_SCREENSHOTS=1` is set. The four committed screenshots were
generated in a separate run with that flag.

Additional host-side verification:

```text
cargo audit                       passed (3 allowed unmaintained warnings)
pnpm audit --audit-level=moderate passed (no known vulnerabilities)
cargo loco doctor (production)    production settings safe; DB connected
scripts/deploy.sh validate        configuration valid
package.sh --check                package layout complete
six native archives               SHA-256 sidecars verified
macOS ARM64 archive               rebuilt from current source (17,866,768-byte
                                  binary, 8,024,060-byte archive, SHA-256
                                  2948857cd102); layout and checksum verified
workflow YAML                     parsed; manual-only, pinned SHAs
```

Local bounded load test (50 sequential requests each, disposable data):
item list 4.8 ms, item create 5.8 ms, dashboard stats 9.7 ms average. These
are development-process figures on one host, not a capacity guarantee.

### Release classification

**READY WITH LIMITATIONS.** All automated checks pass and the audited high and
medium correctness issues are fixed. The limitations below are documented and
accepted for v0.1.0; none blocks an authorized tag or publication, but the
operator must understand them.

### Decisions

- **Keep the generated JWT-in-`localStorage` flow for v0.1.0.** A partial
  cookie migration was rejected; strict CSP, security headers, no-store and
  service-worker discipline are compensating controls. The residual XSS/token
  theft risk is documented in `SECURITY.md` and the release notes.
- **`preset: empty` with explicit headers.** Loco's bundled presets can clear
  browser storage or weaken PWA/static caching; explicit overrides are
  predictable and version-stable.
- **Login failures only.** A known email must not be an account lockout vector;
  the successful login clears the counter, and login uses an account+client
  dimension. This replaces the earlier global hard lock.
- **HSTS at the proxy.** The application cannot know whether external TLS is
  real, so the app does not emit HSTS; the deployment guide requires it at the
  TLS terminator.
- **Generic registration response.** Returning `409` would be a reliable
  account oracle when registration is enabled; neutral success preserves the
  SPA copy and the audit finding is resolved.
- **Local archives are verification artifacts.** The checked-in `dist/` archives
  were built before Task 007 (their `RELEASE` metadata records an earlier git
  SHA). The manual release workflow rebuilds every archive from the tagged
  commit before publication, so no stale binary is published.

### Blockers

None that prevent an authorized release. Tag, push, image publication and
deployment remain explicitly unauthorized and were not performed.

### Known limitations

- JWT in `localStorage`: any XSS is a potential session compromise; CSP reduces
  but does not remove the risk. Seven-day stateless JWTs cannot be revoked.
- Authentication and test-send rate limits are in-process, reset on restart,
  and depend on correct `REMOTE_IP_SOURCE`; public exposure also needs edge
  limits. Only one application replica is supported.
- `style-src 'unsafe-inline'` remains for Mantine runtime styles.
- Most local `dist/` archives predate Task 007; the macOS ARM64 one was rebuilt
  from current source for verification, and the authorized release workflow
  rebuilds all six from the tagged commit before publication. Only the macOS
  ARM64 package, `linux/arm64` Docker image and musl ARM64 package were booted
  here; macOS AMD64, both Linux AMD64 binaries and glibc ARM64 were
  cross-compiled but not runtime-tested.
- No SSH deployment or real service boot was executed against a target.
- Application and queue SQLite databases are backed up sequentially; the queue
  is recoverable but not transactional with the application database.
- Sub-2 MiB idle RSS figures are for a single idle replica; they are not a
  capacity guarantee under sustained concurrent load.

## Next task

None. All milestones are complete. Continue only with explicit authorization
for tagging, publishing or deployment.
