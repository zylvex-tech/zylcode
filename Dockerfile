# syntax=docker/dockerfile:1
# ---------------------------------------------------------------------------
# ZylCode development image
#
# One reproducible toolchain for a fresh clone. Three things have to agree
# for a container build to mean anything:
#
#   1. Rust — pinned to `rust-version` from the workspace manifest (the MSRV
#      floor a contributor is told about), with clippy and rustfmt because
#      CI treats both as gates.
#   2. Node/pnpm — Node 22 (the version .github/workflows/ci.yml runs) and
#      pnpm taken from `packageManager` in package.json rather than guessed,
#      so the lockfile is interpreted by the same resolver that wrote it.
#   3. System libraries — the exact GTK/WebKit set the CI Linux runner
#      installs, because `apps/zylcode-desktop/src-tauri` is a workspace
#      member and will not compile without it.
#
# Used by both `.devcontainer/devcontainer.json` and `docker-compose.yml`.
# ---------------------------------------------------------------------------

FROM ubuntu:24.04

# Overridable so the image can be rebuilt against a different floor without
# editing the file: `docker build --build-arg RUST_TOOLCHAIN=1.91.0 .`
#
# The default is `rust-version` from Cargo.toml (1.90), verified with
# `cargo +1.90.0 check --workspace --all-targets`. Do not "simplify" this to
# 1.89 or to the number in an older doc: the Tauri 2.12 dependency tree
# declares rustc 1.90 as its floor, and an older pin builds a broken image.
ARG RUST_TOOLCHAIN=1.90.0
ARG NODE_VERSION=22
ARG PNPM_VERSION=9.12.0

ENV DEBIAN_FRONTEND=noninteractive \
    RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH

# --- system dependencies ---------------------------------------------------
# Mirrors the "Install Linux dependencies (Tauri)" step in ci.yml, plus the
# tools the repository's own checks invoke: git (Repository Intelligence
# derives co-change evidence from `git log`), python3 (the retracted-claim
# and secret-scan gates), patchelf (cargo-llvm-cov), gnupg (NodeSource key).
RUN apt-get update \
 && apt-get install -y --no-install-recommends \
    build-essential \
    ca-certificates \
    curl \
    file \
    git \
    gnupg \
    libayatana-appindicator3-dev \
    libgtk-3-dev \
    libssl-dev \
    librsvg2-dev \
    libwebkit2gtk-4.1-dev \
    patchelf \
    python3 \
    wget \
 && rm -rf /var/lib/apt/lists/*

# --- Rust ------------------------------------------------------------------
# `--profile minimal` keeps the image honest about size; clippy and rustfmt
# are added because the documented check suite runs both.
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --no-modify-path --profile minimal --default-toolchain "${RUST_TOOLCHAIN}" \
 && rustup component add clippy rustfmt \
 && rustc --version

# --- Node + pnpm -----------------------------------------------------------
# NodeSource rather than Ubuntu's own nodejs package: 24.04 ships Node 18,
# which predates the `--experimental-strip-types` flag the N-ATLAS smoke test
# needs (ci.yml pins Node 22 for exactly that reason).
RUN install -m 0755 -d /etc/apt/keyrings \
 && curl -fsSL https://deb.nodesource.com/gpgkey/nodesource-repo.gpg.key \
    | gpg --dearmor -o /etc/apt/keyrings/nodesource.gpg \
 && echo "deb [signed-by=/etc/apt/keyrings/nodesource.gpg] https://deb.nodesource.com/node_${NODE_VERSION}.x nodistro main" \
    > /etc/apt/sources.list.d/nodesource.list \
 && apt-get update \
 && apt-get install -y --no-install-recommends nodejs \
 && rm -rf /var/lib/apt/lists/* \
 && corepack enable \
 && corepack prepare "pnpm@${PNPM_VERSION}" --activate \
 && node --version && pnpm --version

WORKDIR /workspace