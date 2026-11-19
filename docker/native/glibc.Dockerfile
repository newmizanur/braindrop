# syntax=docker/dockerfile:1.7
#
# Toolchain for glibc Linux release builds. The binary is built for the
# container's own architecture (no cross linker), so an amd64 build runs under
# Docker emulation on an Apple Silicon host. Debian bullseye keeps the glibc
# baseline at 2.31 (Ubuntu 20.04+); see docs/DEPLOYMENT.md.
FROM rust:1-bullseye

# The base image already ships `cc`, `pkg-config` and a CA bundle, so no
# `apt-get` is needed. Debian 11 is past LTS and `bullseye-security` has moved
# to archive.debian.org, which would make an `apt-get update` fail here.

WORKDIR /app
