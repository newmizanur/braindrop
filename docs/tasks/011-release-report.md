# Task 011 — v0.1.0 Release Preparation Report

**Status:** Source/tag and repository metadata are published. A GitHub release
draft contains all native archives and checksums. Docker tags and public
release publication await GHCR authentication with a classic PAT and public
package visibility.

**Release source commit:** `86faa3dfa39bb1bf8ccd8e5f96ef71706ca8d302`
(`release: prepare BrainDrop v0.1.0`). The release-source commit is the revision
to tag and use for all release artifacts. This report and its checklist update
are committed afterward as documentation-only follow-up; they do not change the
application or release artifacts.

## Summary

Prepared the README, changelog and release notes; checked version consistency,
security-sensitive tracked content and screenshots; ran full local Docker CI;
built and checksum-verified all six native archives; locally built and tested
the linux/amd64 and linux/arm64 production Docker image variants. The requested
source and archive revision is `86faa3d`. The source/docs `main` branch, the
annotated tag, and repository description/topics were published after the user
authorized publication. All 13 native archive/checksum files are uploaded to a
draft release. GHCR image tags could not yet be published.

## Source, security and repository review

- Scanned tracked content and 850 unique Git-history blobs with redacted
  high-confidence patterns for private keys, common API tokens, credential
  assignments, production environment/database/backup/log artifacts and
  deployment identifiers. No real secrets, production data, personal VPS
  address, or personal deployment domain was found. Pattern matches were
  provider URL shapes, placeholder values, and mock test fixtures. This was a
  manual pattern scan, not a formal Gitleaks run; it cannot prove absence of
  every possible secret.
- Reviewed existing screenshots. They show fictional/example inbox content and
  mock integration states, with no observed personal information or
  credentials.
- `.gitignore` and `.dockerignore` exclude local release output, archives,
  logs, backups and macOS metadata. `dist/` is ignored and release artifacts
  were not staged. No `.env`, SQLite database, backup or application log is
  tracked. `.env.example` contains placeholders only.
- Checked application version `0.1.0` in Cargo metadata/lockfile, frontend
  metadata, application version reporting, changelog and release notes. Docker
  image labels carry `v0.1.0`, source repository, `86faa3d` and `MIT`.
- The local Git remote is `git@github.com:newmizanur/braindrop.git`. The public
  repository was empty before publication. `main` now resolves to
  `fed640324c6f53316b5245aadfd9fc259c86d094`; annotated tag `v0.1.0` resolves
  to tag object `161b0611b44d023ea8cb08adc5d468afc326f3c6`, peeled to validated
  source `86faa3dfa39bb1bf8ccd8e5f96ef71706ca8d302`. GitHub CLI is now
authenticated as `newmizanur` with `repo` and `write:packages` scopes. The
repository description and all 12 requested topics are verified. A draft
release is associated with tag `v0.1.0` and contains 13 uploaded assets; it is
not public until published.

## CI and dependency audits

`./scripts/ci.sh` **PASS** on `86faa3d` (log captured at
`/tmp/task011-ci.log`):

- Rust formatting, Clippy, migrations and tests passed: 104 unit tests, 103
  integration/model/task tests and 29 TypeScript-binding drift tests.
- Frontend type-check, lint, build and 36 unit tests passed.
- Production image health, worker/scheduler startup, database persistence over
  restart, static asset serving, backup/restore and non-root package/runtime
  checks passed.
- Playwright: 14 passed; the opt-in screenshot capture test was skipped.

`cargo audit --no-fetch` found no known vulnerable dependency. It reported
three unmaintained upstream crates with no fixed upgrade available:
`chrono 0.4.45` (RUSTSEC-2026-0543), `chrono-tz 0.10.4`
(RUSTSEC-2026-0555), and `unsafe-libyaml 0.2.11` (RUSTSEC-2024-0449). The
local advisory database was used; crates.io index lock acquisition failed.
No dependencies were changed for these maintenance advisories.

The project-pinned `corepack pnpm@10.12.4 --dir frontend audit
--audit-level=moderate` was retried. It could not reach the npm audit endpoint
because registry DNS returned `ENOTFOUND`; frontend advisory status is therefore
unverified. The first attempt with the system pnpm 12.18.0 was rejected by the
project package-manager version guard and was not treated as an audit result.

## Native release archives

All six archives were built and packaged from `86faa3d` using the existing
scripts. File architecture/linkage, executable mode, runtime config, compiled
frontend, public documentation/license, absence of `.env` and macOS metadata,
archive layout, SHA-256 sidecars and consolidated `dist/SHA256SUMS` were
verified. All six checksum entries return `OK`.

| Archive | Size (bytes) | SHA-256 |
| --- | ---: | --- |
| `braindrop-linux-amd64-musl.tar.gz` | 9,038,254 | `5f7aa88581032712b57a7edb54c4cbe1cf8e0bdc6e0d43d4b2286b71a33d211e` |
| `braindrop-linux-arm64-musl.tar.gz` | 8,309,561 | `ac441162620859c5558e055ec72dc00d650f0f5e6330cf515fec7bb7c4b987b1` |
| `braindrop-linux-amd64-glibc.tar.gz` | 8,821,997 | `101b0a72ff56e5e2b1de744f58ae88a4cef076196285be9fe8dcb73576c91a12` |
| `braindrop-linux-arm64-glibc.tar.gz` | 8,347,545 | `3a96f1000ed93aec1f92f195d4ad8184aae517783133c51525b0c56582e46314` |
| `braindrop-macos-arm64.tar.gz` | 8,028,290 | `3f7fcb92130c550053350b5069313c69e4a139f50024db439deccab277ffabf7` |
| `braindrop-macos-amd64.tar.gz` | 8,566,862 | `2a4889a48f7484d2376b1d2c30ad2a1763a81a71c2344addf3132e022b8e9b22` |

Native artifacts were cross-built for all targets. Linux x86_64 musl was exercised
in the native Alpine deployment validation recorded under Task 010; the macOS
ARM64 binary was invoked on this ARM64 development host. The other native
artifact targets were not independently runtime-tested. Docker images for both
Linux architectures were run below; this does not constitute runtime testing of
the native archives for those architectures.

## Docker image preparation and validation

Built locally from the existing multi-stage Dockerfile with
`VERSION=v0.1.0`, `BUILD_SHA=86faa3d`, target `runtime`:

| Local validation tag | Platform | Image size |
| --- | --- | ---: |
| `ghcr.io/newmizanur/braindrop:v0.1.0-amd64` | linux/amd64 | 43,972,352 bytes |
| `ghcr.io/newmizanur/braindrop:v0.1.0-arm64` | linux/arm64 | 43,379,407 bytes |

Both carry OCI source, version, revision and MIT license labels. On Docker
Desktop running on the ARM64 development host, each image ran as UID 10001 and
passed health, root SPA, deep SPA route, static asset, application/queue SQLite
creation, storage persistence over container restart, and worker/scheduler
startup checks using disposable test volumes. The amd64 check ran under
emulation. Docker CI also passed its production-image and Playwright checks.

These are platform-specific local validation tags, not final registry tags. A
redundant OCI archive was not generated because free host disk space fell below
250 MiB after builds. A later Buildx multi-platform publish attempt failed
before publishing tags: amd64/arm64 concurrently unpacked the same Cargo crate
into a shared cache. The fallback push of the validated platform images
uploaded some layers, then the registry denied the image manifest. The package
is private, unlinked to the repository, and has no published `v0.1.0` or
`latest` tag. GitHub documents that Container Registry authentication requires
a classic PAT; the `gh` OAuth token cannot publish this package. The package
settings UI is also required to link it to the public repository and make it
public. No final image tag or manifest is public.

## Release notes and known limitations

Release notes are in [`docs/releases/v0.1.0.md`](../releases/v0.1.0.md), with
concise installation and feature documentation in README. They describe manual
capture and optional BYOK AI, notifications, single-replica SQLite, localStorage
JWT risk, at-least-once notification delivery and hosting-specific limitations.

Task 010's read-only HTTPS verification addendum is preserved in the Task 010
report. Health, root and SPA routing worked through the public HTTPS edge;
registration remained disabled and unauthenticated item access returned 401.
Authorized-user CORS/authentication was not verified. Cloudflare SSL/TLS mode
and encryption from Cloudflare to origin remain unknown; trusted proxy header
semantics across Cloudflare and managed HAProxy are unverified. No Cloudflare,
DNS, HAProxy or application deployment settings were changed for this task.

Other applicable limitations: one application replica; SQLite storage and
queue; in-memory rate-limit state; at-least-once notifications; Fly auto-stop
pauses workers and schedules; remote pending-notification recovery not fully
validated; TierHive reboot/authenticated item/notification recovery checks
remain incomplete; native non-host platforms were cross-built only; and no
public demo or permanent public administrator is being announced. These are
recorded in the release notes and Task 010 report. No production-capacity claim
is made from the short benchmark samples.

## Publication status and remaining steps

The user authorized all publication actions. The validated source branch and
tag are pushed, metadata is updated, and the release draft contains all 13
assets. Complete the following after GHCR package access is fixed.

1. **Complete:** push source/docs branch and create the release tag. Verified
   with `git ls-remote`; `main` contains the report commits and the tag peels
   to the release source commit.

   ```sh
   git push origin main
   git tag -a v0.1.0 86faa3dfa39bb1bf8ccd8e5f96ef71706ca8d302 -m "BrainDrop v0.1.0"
   git push origin v0.1.0
   ```

2. **Pending:** use a classic PAT with `write:packages` for Docker login,
   link the package to this repository and set visibility to Public in the
   package settings UI. Then push the already-validated platform images and
   assemble each multi-platform manifest:

   ```sh
   docker push ghcr.io/newmizanur/braindrop:v0.1.0-amd64
   docker push ghcr.io/newmizanur/braindrop:v0.1.0-arm64
   docker manifest create ghcr.io/newmizanur/braindrop:v0.1.0 \
     ghcr.io/newmizanur/braindrop:v0.1.0-amd64 \
     ghcr.io/newmizanur/braindrop:v0.1.0-arm64
   docker manifest push ghcr.io/newmizanur/braindrop:v0.1.0
   docker manifest create ghcr.io/newmizanur/braindrop:latest \
     ghcr.io/newmizanur/braindrop:v0.1.0-amd64 \
     ghcr.io/newmizanur/braindrop:v0.1.0-arm64
   docker manifest push ghcr.io/newmizanur/braindrop:latest
   ```

3. **Draft created:** after images are public, publish the existing release
   draft: `gh release edit v0.1.0 --draft=false`. It contains all six archives,
   six SHA-256 sidecars, and `SHA256SUMS`.

4. **Complete:** repository description and all 12 requested topics were
   verified through `gh repo view`.

## Commits and working tree

- Release source: `86faa3dfa39bb1bf8ccd8e5f96ef71706ca8d302`.
- The release report/checklist follow-up commit is recorded in Git after this
  file is committed. It contains documentation only.
- Existing Git author identity is preserved; implementation commits include
  Codex co-author attribution.
- No secrets or local release artifacts are added to Git. `dist/` remains
  ignored.

## Readiness assessment

**PARTIALLY PUBLISHED; NOT COMPLETE.** `main`, the source-pinned tag,
repository metadata, and a draft with all assets are verified. Public Docker
tags and public release publication remain blocked until the GHCR package is
linked and made public in GitHub settings and a classic PAT is used for pushes.
After that, anonymous pulls, manifest digests, and public asset URLs must be
checked. Frontend advisory scanning, several native runtime targets, public
authenticated proxy/CORS behavior, origin TLS mode, and remote/sustained
notification recovery remain unverified. Do not mark Task 011 complete until
the remaining publication checks pass.
