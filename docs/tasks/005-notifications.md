# Task 005 — Notifications

**Status:** Complete

## Objective

Deliver scheduled reminders, daily digests and overdue-task notifications through
Telegram and/or Discord, with durable delivery state, deduplication, bounded
retries and safe handling of credentials.

## Scope

- `notification_deliveries` migration/model.
- Encrypted storage for Telegram token and Discord webhook.
- Provider clients behind a small common interface.
- `POST /api/notifications/test`.
- `NotificationDeliveryWorker` on the SQLite queue.
- Scheduled scans: reminders (minute), digest eligibility (minute), overdue
  (hourly), cleanup (daily).
- Restart recovery for stale `processing` deliveries.
- Settings UI for channel configuration and notification preferences.
- Tests with mocked providers; no real network in CI.

## Dependencies

- Task 003 complete (items/settings).
- Task 002 recommended (auth policy and verified accounts).

## Implementation checklist

- [x] Add `notification_deliveries` model/migration with unique dedup key and
      status/next-attempt indexes.
- [x] Add encrypted credential storage and an externally configured key.
- [x] Add notification config and per-user preferences.
- [x] Implement Telegram and Discord clients with timeouts and bounded retries.
- [x] Restrict outbound URLs to approved provider endpoints (SSRF).
- [x] Implement the delivery worker and status transitions.
- [x] Implement scheduled scan tasks and `scheduler:` entries.
- [x] Compute user-local digest eligibility from timezone.
- [x] Implement `POST /api/notifications/test`.
- [x] Recover stale `processing` deliveries after restart.
- [x] Build the Integrations and notification settings UI.
- [x] Add tests: delivery success/failure, dedup, retries, restart recovery,
      secrets never returned/logged.
- [x] Update docs and PROGRESS.

## Acceptance criteria

- A user can enable Telegram, Discord, both or neither.
- A test notification reaches the configured channel.
- Due reminders are enqueued once and delivered; duplicate scheduler runs do not
  enqueue duplicates.
- Failed deliveries retry with a bounded backoff and end in `failed` with a
  sanitized error.
- Restarting the app recovers pending/stale deliveries.
- Credentials are encrypted, never returned by the API and never logged.
- CI passes with mocked providers and no credentials.

## Validation commands

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --workspace

pnpm --dir frontend typecheck
pnpm --dir frontend lint
pnpm --dir frontend test
pnpm --dir frontend build

cargo loco scheduler --list
```

## Out of scope

- Email notifications (Loco auth mail is separate).
- Push notifications / web push.
- Telegram bot gateway or Discord gateway connections.
- Exactly-once delivery guarantees.
- Arbitrary webhooks or user-supplied hosts.
- Multi-replica queue coordination.
