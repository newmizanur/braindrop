# Security Policy

## Supported versions

| Version | Supported |
|---|---|
| `v0.1.0` | Supported — first release |
| Earlier or unreleased builds | Not supported |

Run a released version, apply updates, and keep to a single application replica
(SQLite is single-writer). Security reports are triaged against `v0.1.0`.

## Reporting a vulnerability

Please **do not open a public issue** for a suspected vulnerability. Email the
maintainer privately at **newmizanur@gmail.com** with:

- the affected version or commit;
- steps to reproduce and the observed impact;
- any relevant configuration (with secrets redacted);
- a suggested fix, if you have one.

Do not include real API keys, bot tokens, webhook URLs, `.env` contents or user
data in a report. Redact them before sending.

### Response expectations

These are targets, not guarantees:

- **Acknowledgement** within 3 working days of the report.
- **Initial triage** within 7 working days, including an assessment of severity
  and affected versions.
- **Progress updates** at least every 7 working days while a fix is prepared.
- **Coordinated disclosure**: we ask that you give us a reasonable window to
  investigate and release a fix before public disclosure. We will credit
  reporters who wish to be named.

If you receive no acknowledgement, follow up once after 7 working days in case
the message was missed.

## Scope of the review

BrainDrop is a self-hosted, single-node application. The operator owns the TLS
termination point, the persistent SQLite files and the environment file. The
v0.1.0 hardening covers authenticated ownership on every data query, typed
validation, secret handling, outbound destination restrictions, browser caching,
security headers and authentication rate limits. Local release validation has
passed, with documented platform and remote-validation limits; this document
describes the implemented posture, not a certification.

### Public Internet exposure

The service binds `0.0.0.0:5150` by default in the production image. If `HOST`
is a public origin or the port is forwarded, **the application is reachable from
the Internet**, including its authentication and registration endpoints.

Before exposing it:

- terminate TLS at a trusted reverse proxy or platform load balancer and set
  `HOST` to the public **HTTPS** origin;
- restrict or close the application port at the host firewall so only the proxy
  can reach it;
- keep `ALLOW_REGISTRATION=false` unless public sign-up is intended, and create
  accounts with `admin:create`;
- keep mail disabled unless verification/reset flows are needed;
- add rate limiting at the edge as well (see below);
- back up the SQLite files and the encryption keys, and store `.env` with mode
  `600` outside any published artifact.

### Authentication and rate limiting

Loco's generated JWT authentication is used unchanged. v0.1.0 adds fixed-window,
cache-backed limits for `login`, `register`, `forgot`, `reset`, `verify`,
`magic-link`, `magic-link-verify` and `resend-verification`, keyed by both the
account/token and the client address. Unknown-email logins use a dummy password
hash to keep response times close to a real check.

The counters live in the in-process cache: they are shared by one process and
**reset on restart**. They fail open if the cache is unavailable. This is a
reasonable defence for a single-replica personal installation, not a substitute
for edge rate limiting on a public endpoint. If you expose BrainDrop to the
Internet, enable request limits at your reverse proxy as well.

### Browser storage and XSS

The generated frontend stores the JWT in `localStorage`. This is a **residual
risk**: any successful XSS in the SPA can read that token and act as the user
until it expires or the signing secret is rotated. Logout clears the client
copy; it does not revoke a token already copied elsewhere. Treat any XSS report
as a potential token-compromise incident and rotate `JWT_SECRET` if needed to
invalidate all sessions.

Compensating controls in v0.1.0:

- a strict Content-Security-Policy (`default-src 'self'`, `script-src 'self'`,
  `script-src-attr 'none'`, no `unsafe-eval` or inline scripts, `object-src
  'none'`, `base-uri 'none'`, `frame-ancestors 'none'`, `form-action 'self'`);
  the pre-paint colour-scheme code lives in an external file so no inline script
  is required;
- `X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY`,
  `Referrer-Policy: no-referrer`, `Permissions-Policy`,
  `Cross-Origin-Opener-Policy: same-origin` and
  `Cross-Origin-Resource-Policy: same-origin`;
- `Cache-Control: no-store` on `/api/*`, monitoring paths (`/_*`) and HTML
  documents, while hashed static assets stay cacheable;
- a service worker that caches only the app shell and static assets and never
  caches `/api/*`, `/_*` or non-GET requests;
- no provider credentials in API responses, logs or browser storage.

Style requests still allow `'unsafe-inline'` for Mantine's runtime styles;
script execution does not.

### HSTS and trusted proxies

BrainDrop does **not** emit `Strict-Transport-Security`. The application cannot
see whether TLS was terminated upstream, so HSTS must be configured on the
reverse proxy or load balancer. Do not rely on the app for it.

Client addresses for rate limiting come from Loco's `remote_ip` middleware.
`REMOTE_IP_SOURCE` defaults to `ConnectInfo` (the socket peer), which is correct
when the app is exposed directly or only through a proxy that preserves the
connection address. Set it to `RightmostXForwardedFor` **only** behind exactly
one trusted reverse proxy that overwrites `X-Forwarded-For` with the real client
address. Never trust an `X-Forwarded-For` header that a client can supply, and
do not expose the application port publicly when using header-based sources.

### Integrations and secrets

- Core capture, tasks, notes, ideas and reminders need **no AI, Telegram or
  Discord credentials**. Keep `AI_ENABLED=false` and channels off if unused.
- `JWT_SECRET` must be base64 (`openssl rand -base64 32`); Loco decodes it
  before signing. `AI_ENCRYPTION_KEY` and `NOTIFICATION_ENCRYPTION_KEY` are
  long-lived: set them once, back them up, and never let a deploy regenerate an
  existing key. Rotating them makes stored personal keys or channel credentials
  unreadable.
- Personal OpenRouter keys and Telegram/Discord credentials are sealed with
  AES-256-GCM and are write-only in the API.
- The OpenRouter client disables redirects so an operator-configured base URL
  cannot redirect a request carrying a key to another host. Treat
  `OPENROUTER_BASE_URL` as trusted configuration.
- Telegram and Discord destinations are restricted to official HTTPS hosts
  with redirects disabled; there is no arbitrary webhook URL feature.
- Notification history is sanitized, and database errors are redacted before
  they reach an API response or a delivery record.
- Delivery is **at-least-once**: a retry after an ambiguous provider failure can
  produce a duplicate message. Deduplication keys prevent duplicate scheduler
  enqueues, not provider-side duplicates.

### Deployment hygiene

The SSH release flow uses only `ssh`/`scp`, keeps host-key verification enabled,
stages uploads under the SSH user's home with an unpredictable `mktemp` name and
`umask 077`, marks the staging directory `0700`, and removes the uploaded
archive and staging directory on both success and failure. It never uploads a
local `.env` and never regenerates an existing encryption key. Backups use
SQLite's online backup API, never `cp` on a live database.

## License

Distributed under the [MIT License](LICENSE).
