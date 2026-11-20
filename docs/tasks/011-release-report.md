# Task 011 — BrainDrop v0.1.0 Release Report

**Status:** Published and publicly verified; GHCR repository association remains outstanding.

**Validated release source:** `86faa3dfa39bb1bf8ccd8e5f96ef71706ca8d302`

**Public release:** <https://github.com/newmizanur/braindrop/releases/tag/v0.1.0>

The immutable `v0.1.0` tag remains pinned to the validated source revision.
Documentation follow-up commits are on `main` and do not move the tag.

## Table of Contents

<!-- toc:start -->
- [Publication results](#publication-results)
- [Native archives](#native-archives)
- [Security, CI, and limitations](#security-ci-and-limitations)
- [Repository and metadata](#repository-and-metadata)
- [Final verification still required](#final-verification-still-required)
<!-- toc:end -->
## Publication results

- The `main` branch and annotated `v0.1.0` tag were pushed. The tag peels to
  `86faa3dfa39bb1bf8ccd8e5f96ef71706ca8d302`.
- The GitHub release was published and marked latest. Its uploaded set is six
  native archives, six per-archive SHA-256 sidecars, and `SHA256SUMS`.
- `ghcr.io/newmizanur/braindrop:v0.1.0` and `:latest` were published as
  multi-platform OCI manifests. Both resolved to
  `sha256:74e241438c1e61ea70acc62e3a7dca484e34c7c4b2f5aad31c987c96ce41a4f1`
  on live anonymous inspection. The index contains linux/amd64 manifest
  `sha256:07ee75e95f026f7da6dcf12975882a1b0e98ae3044defa9f7c4b6d8ada30c6b4`
  and linux/arm64 manifest
  `sha256:6c2df4e4cd37698a90f45fc08a356001efa000b055e7e4985f5086d4044dae1a`.
  Anonymous pulls of both platforms succeeded for `v0.1.0`; `latest` resolves to
  the same multi-platform index digest.
- GHCR package visibility was set to public. The latest authenticated package
  API result available in the interrupted publication session had
  `repository: null`; repository association therefore remains unverified and
  appears not to be connected. The owner can connect it from the package page's
  **Connect repository** setting.
- Live anonymous download of the public release API and every archive,
  sidecar, and `SHA256SUMS` succeeded. All six archives passed both the
  consolidated checksum file and their individual SHA-256 sidecars. The GitHub
  release API reports `draft: false` and `prerelease: false`.

## Native archives

All six archives were built from the validated source revision. Local archive
checksums and sizes were verified before upload. Linux x86_64 musl was exercised
on Alpine, and macOS ARM64 was invoked on the development host. The other native
targets were cross-built but not independently runtime-tested.

| Archive | Size (bytes) | SHA-256 |
| --- | ---: | --- |
| `braindrop-linux-amd64-musl.tar.gz` | 9,038,254 | `5f7aa88581032712b57a7edb54c4cbe1cf8e0bdc6e0d43d4b2286b71a33d211e` |
| `braindrop-linux-arm64-musl.tar.gz` | 8,309,561 | `ac441162620859c5558e055ec72dc00d650f0e5e6330cf515fec7bb7c4b987b1` |
| `braindrop-linux-amd64-glibc.tar.gz` | 8,821,997 | `101b0a72ff56e5e2b1de744f58ae88a4cef076196285be9fe8dcb73576c91a12` |
| `braindrop-linux-arm64-glibc.tar.gz` | 8,347,545 | `3a96f1000ed93aec1f92f195d4ad8184aae517783133c51525b0c56582e46314` |
| `braindrop-macos-arm64.tar.gz` | 8,028,290 | `3f7fcb92130c550053350b5069313c69e4a139f50024db439deccab277ffabf7` |
| `braindrop-macos-amd64.tar.gz` | 8,566,862 | `2a4889a48f7484d2376b1d2c30ad2a1763a81a71c2344addf3132e022b8e9b22` |

## Security, CI, and limitations

The release preparation scan covered tracked files and Git history with
redacted high-confidence patterns. No secrets, production data, or personal
server identifiers were found. This was not a formal secret scanner and cannot
prove absence of every possible secret. `.gitignore` and `.dockerignore` cover
local secrets and generated release data; the screenshots contain fictional
sample data.

`./scripts/ci.sh` passed on the release source: Rust checks and tests, frontend
typecheck/lint/tests/build, production image checks, and Playwright smoke tests.
Rust audit reported no known vulnerable dependency and three upstream
unmaintained-crate advisories without a suitable fixed upgrade. A fresh pnpm
audit could not reach npm registry DNS (`ENOTFOUND`), so current frontend
advisory status is unknown.

Known limitations remain: one application replica; SQLite storage and queue;
JWT tokens stored in browser local storage; at-least-once notification delivery;
Fly auto-stop pauses workers and schedules; sustained capacity and some remote
notification recovery are not validated; not all native targets were runtime
tested; and TierHive's authenticated proxy/CORS workflow and origin TLS mode
remain unverified. No public demo URL is advertised and registration remains
disabled on existing deployments.

## Repository and metadata

The public repository is <https://github.com/newmizanur/braindrop>. Its
requested description and 12 topics were set during publication. README,
license, screenshots, installation links, and release documentation are present.
GitHub Actions were not triggered.

## Final verification still required

1. Connect the GHCR package to `newmizanur/braindrop` in package settings. The
   latest available package API response had `repository: null`. Re-authenticate
   `gh` if API verification or settings changes are needed; current local CLI
   credentials are invalid.
2. The documentation-only follow-up is pushed to `main` as `e487b3f`. The
   immutable `v0.1.0` tag was not moved.

Tasks 001–010 and the publication, asset, and anonymous-pull portions of Task
011 are complete. The repository association remains unverified and appears
unset, so Task 011 should not be marked complete until that is resolved.
