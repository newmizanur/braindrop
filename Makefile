# BrainDrop build and deployment entry points.
#
# Run `make help` for the list. The deployment targets use plain SSH/SCP; no
# rsync, Docker or compiler is required on the remote host.

SHELL := /usr/bin/env bash
.SHELLFLAGS := -eu -o pipefail -c

DIST ?= dist
SSH_HOST ?=
SSH_PORT ?= 22
SSH_USER ?= root
REMOTE_DIR ?= /opt/braindrop
OS ?= alpine
ARCH ?= amd64
SERVICE ?= braindrop
SERVICE_USER ?= braindrop
APP_PORT ?= 5150
KEEP_RELEASES ?= 3
SKIP_BUILD ?= 0
AUTO_ROLLBACK ?= 1
GENERATE_SECRETS ?= 1
TARGET ?= $(shell rustc -vV 2>/dev/null | sed -n 's/^host: //p')
FLY_CLI ?= fly
FLY_APP ?= braindrop
FLY_REGION ?= sin
FLY_VOLUME ?= braindrop_data
FLY_VOLUME_SIZE ?= 1
FLY_MEMORY ?= 256

export DIST SSH_HOST SSH_PORT SSH_USER SSH_IDENTITY REMOTE_DIR OS ARCH \
	SERVICE SERVICE_USER APP_PORT KEEP_RELEASES SKIP_BUILD AUTO_ROLLBACK \
	GENERATE_SECRETS RELEASE_ID ROLLBACK_RELEASE LINES TARGET

.PHONY: help run build build-frontend build-linux build-linux-amd64 \
	build-linux-arm64 build-alpine build-alpine-amd64 build-alpine-arm64 \
	build-mac build-all package package-check ci fmt test clean \
	deploy-files deploy-init deploy-linux deploy-restart deploy-status \
	deploy-logs deploy-stop deploy-backup deploy-rollback \
	fly-init fly-deploy fly-status fly-logs fly-ssh fly-restart

define HELP_TEXT
BrainDrop targets

  Local development
    make run                  Build the SPA and run server + worker + scheduler
    make build                Build the frontend and host release binary
    make build-frontend       Build frontend/dist only
    make test                 Run Rust and frontend unit tests
    make fmt                  Format Rust code
    make ci                   Run the full Docker CI pipeline
    make clean                Remove build output (keeps databases and .env)

  Release builds (Docker cross-compilation for Linux)
    make build-linux          Ubuntu glibc AMD64 + ARM64
    make build-linux-amd64    x86_64-unknown-linux-gnu
    make build-linux-arm64    aarch64-unknown-linux-gnu
    make build-alpine         Alpine musl AMD64 + ARM64
    make build-alpine-amd64   x86_64-unknown-linux-musl
    make build-alpine-arm64   aarch64-unknown-linux-musl
    make build-mac            macOS Apple Silicon + Intel
    make build-all            All six supported targets
    make package              Package TARGET (default: this host's target)
    make package-check        Verify an existing package layout
                              (DIR=dist/braindrop-linux-amd64-musl)

  SSH deployment (OS=alpine|ubuntu, ARCH=amd64|arm64)
    make deploy-init          First install: user, directories, .env, service
    make deploy-files         Upload a release without activating it
    make deploy-linux         Upload, migrate, activate, health-check
    make deploy-restart       Restart the service
    make deploy-status        Service state, current release and health
    make deploy-logs          Follow service logs
    make deploy-stop          Stop the service
    make deploy-backup        Run scripts/backup.sh on the remote host
    make deploy-rollback      Switch to the previous release and restart

  Fly.io Docker deployment (requires flyctl installed and authenticated)
    make fly-init             Create the app/volume only when absent
    make fly-deploy           Deploy; refuses apps with more than one machine
    make fly-status           Show app and machine status
    make fly-logs             Follow application logs
    make fly-ssh              Open a console on the application machine
    make fly-restart          Restart the application's running machine

  Required for deployment: SSH_HOST and SSH_USER.
  Optional: SSH_PORT (22), REMOTE_DIR (/opt/braindrop), SERVICE (braindrop),
            APP_PORT (5150), KEEP_RELEASES (3).
  Fly settings: FLY_APP (braindrop), FLY_REGION (sin),
                FLY_VOLUME (braindrop_data), FLY_VOLUME_SIZE (1 GB),
                FLY_MEMORY (256 MB).
  `fly-init` can create billable Fly.io resources. Review current pricing and
  authenticate with `fly auth login` before running it. No target destroys
  resources or configures secrets.

  Examples:
    make deploy-init SSH_HOST=server.example.com SSH_USER=root OS=alpine
    make deploy-linux SSH_HOST=server.example.com SSH_USER=deploy OS=ubuntu ARCH=arm64
    make fly-init FLY_APP=my-braindrop
    make fly-deploy FLY_APP=my-braindrop
endef

help:
	$(info $(HELP_TEXT))
	@true

run: build-frontend
	cargo loco start --all

build: build-frontend
	cargo build --release --bin braindrop-cli

build-frontend:
	./scripts/build-frontend.sh

build-linux: build-linux-amd64 build-linux-arm64

build-linux-amd64:
	./scripts/build-native.sh x86_64-unknown-linux-gnu

build-linux-arm64:
	./scripts/build-native.sh aarch64-unknown-linux-gnu

build-alpine: build-alpine-amd64 build-alpine-arm64

build-alpine-amd64:
	./scripts/build-native.sh x86_64-unknown-linux-musl

build-alpine-arm64:
	./scripts/build-native.sh aarch64-unknown-linux-musl

build-mac:
	./scripts/build-native.sh aarch64-apple-darwin
	./scripts/build-native.sh x86_64-apple-darwin

build-all: build-linux build-alpine build-mac

package:
	./scripts/package.sh $(TARGET)

package-check:
	@test -n "$(DIR)" || { echo "usage: make package-check DIR=dist/braindrop-<platform>" >&2; exit 2; }
	./scripts/package.sh --check "$(DIR)"

ci:
	./scripts/ci.sh

fmt:
	cargo fmt --all

test:
	cargo test --workspace
	corepack pnpm@10.12.4 --dir frontend test

clean:
	cargo clean
	rm -rf "$(DIST)"
	rm -rf frontend/dist

deploy-files:
	./scripts/deploy.sh files

deploy-init:
	./scripts/deploy.sh init

deploy-linux:
	./scripts/deploy.sh deploy

deploy-restart:
	./scripts/deploy.sh restart

deploy-status:
	./scripts/deploy.sh status

deploy-logs:
	./scripts/deploy.sh logs

deploy-stop:
	./scripts/deploy.sh stop

deploy-backup:
	./scripts/deploy.sh backup

deploy-rollback:
	./scripts/deploy.sh rollback

# Fly commands are deliberately explicit. Initialization is idempotent for
# existing resources; deployment refuses to proceed if more than one machine
# exists, avoiding implicit destruction or an extra SQLite-volume machine.
fly-init:
	@command -v "$(FLY_CLI)" >/dev/null || { echo "flyctl is required (https://fly.io/docs/flyctl/install/)" >&2; exit 127; }
	@$(FLY_CLI) auth whoami >/dev/null
	@if ! $(FLY_CLI) apps list | awk -v app="$(FLY_APP)" 'NR > 1 && $$1 == app { found = 1 } END { exit !found }'; then \
	  $(FLY_CLI) apps create "$(FLY_APP)"; \
	else echo "Fly app $(FLY_APP) already exists; preserving it"; fi
	@if ! $(FLY_CLI) volumes list --app "$(FLY_APP)" | awk -v volume="$(FLY_VOLUME)" 'NR > 1 && $$2 == volume { found = 1 } END { exit !found }'; then \
	  $(FLY_CLI) volumes create "$(FLY_VOLUME)" --app "$(FLY_APP)" --region "$(FLY_REGION)" --size "$(FLY_VOLUME_SIZE)" --yes; \
	else echo "Fly volume $(FLY_VOLUME) already exists; preserving it"; fi
	@echo "Ensure fly.toml app/HOST/mount values match FLY_APP and FLY_VOLUME before deploying. Volume and machine usage may incur charges."

fly-deploy:
	@command -v "$(FLY_CLI)" >/dev/null || { echo "flyctl is required (https://fly.io/docs/flyctl/install/)" >&2; exit 127; }
	@set -- $$($(FLY_CLI) machine list --quiet --app "$(FLY_APP)"); count=$$#; \
	if [ "$$count" -gt 1 ]; then \
	  echo "Refusing deployment: $(FLY_APP) has $$count machines. Inspect and remove extras explicitly before using its single SQLite volume." >&2; exit 2; \
	fi
	$(FLY_CLI) deploy --app "$(FLY_APP)" --ha=false
	$(FLY_CLI) scale memory "$(FLY_MEMORY)" --app "$(FLY_APP)"

fly-status:
	$(FLY_CLI) status --app "$(FLY_APP)"

fly-logs:
	$(FLY_CLI) logs --app "$(FLY_APP)"

fly-ssh:
	$(FLY_CLI) ssh console --app "$(FLY_APP)"

fly-restart:
	$(FLY_CLI) apps restart "$(FLY_APP)"
