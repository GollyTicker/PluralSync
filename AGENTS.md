# Instructions for AI Coding Agents

## What is PluralSync?

A cloud service that syncs plural systems' fronting status across platforms (SimplyPlural, PluralKit, VRChat, Discord, websites). Users update once on their preferred system manager and PluralSync propagates it everywhere else.

Public alpha: [public-test.pluralsync.ayake.net](https://public-test.pluralsync.ayake.net)

Supported sync directions:
* **From SimplyPlural** → any target
* **From PluralKit** → any target except PluralKit
* **From WebSocket Source** -> any target
* **Any target** = VRChat, Discord (Rich Presence + Status), Website, PluralKit

---

## ⚠️ Critical: Use Project Scripts

**This project provides shell scripts in `./steps/` and `./test/`. Always prefer these over writing custom bash commands.**

**Use scripts for:** building, testing, linting, audits, DB migrations, running services, cleaning, releases.
**Use custom commands for:** file ops, git operations, codebase exploration, quick inspections.

**Discovery:** `ls steps/` and `ls test/`. Scripts are numbered (01-clean, 02-install, 10-lint, 12-build, etc.). Read a script before invoking to understand it.

---

## High-Level Architecture

Four main components, all in one Cargo workspace:

| Component | Tech | Purpose |
|---|---|---|
| **`pluralsync`** (backend) | Rust + Rocket + SQLx + Tokio | REST API, WebSocket, user auth (JWT), external API integrations, Prometheus metrics |
| **`frontend`** | Vue 3 + TypeScript + Vite | Main web UI. Communicates with backend via HTTP (axios) and WebSocket |
| **`pluralsync-bridge`** | Tauri 2 (Rust + Vue) | Desktop app for local Discord Rich Presence. Auto-starts on boot, runs independently |
| **`base-src`** | Rust | Shared library (types, utilities) used by both backend and bridge |

**Database:** PostgreSQL 17. Migrations via SQLx (`docker/migrations/`). Secrets encrypted in DB.
**Deployment:** Docker Compose (Nginx reverse proxy → frontend + backend API).

### Sync Flow

1. **Source detects change** — SimplyPlural WebSocket (real-time) or PluralKit HTTP polling
2. **Process** — Privacy rules applied, names cleaned per platform, formatted
3. **Distribute** — Each enabled updater async-pushes to its target platform

User config enables/disables sources and targets (stored encrypted in PostgreSQL).

---

## Coding Guidelines

* **Rust Edition 2024** (requires Rust 1.90.0+)
* **Imports:** One `use` statement per crate. Multiple items from the same crate on one line: `use anyhow::{anyhow, Error, Result}`. Separate lines for different modules.
* **Derive traits:** Prefer `fmt::Display` over `Debug`. Only derive if printing to logs is useful.
* **⛔ Security — Sensitive fields:** Never `#[derive(Debug)]` or `#[derive(Display)]` on structs with sensitive fields (passwords, tokens, API keys, cookies, secrets). If debug output is needed, implement manually and redact all sensitive values.
* **Error handling:** Updaters track `last_operation_error`. Status enum: `Running`, `Disabled`, `Error(msg)`, `Starting`.
* **Avoid comments:** Almost all comments can and should be avoided, since the code should be written in a way that doesn't require comments which simply repeat what's written directly as code. Only comments which explain something non-trivial and non-obvious should be added - after asking for confirmation.
* **KEEP IT SIMPLE AND STUPID:** Use simpler way to implement stuff. Keep it simple. Only add complexity, if it's really necessary. Avoid generic code and extra variables, unless we'll actually use them or they're part of the spec.
* **Bash styles:** Look at existing bash scripts and take the same style. Avoid `local` bash variables. Bash will always run single-threaded in our cases so use simple global UPPERCASE variables.

---

## Essential Commands

> All commands below are scripts in `./steps/` or `./test/`.

| Task | Command |
|---|---|
| Install deps | `./steps/02-install-dependencies.sh` |
| Clean | `./steps/01-clean.sh` |
| Prepare DB | `./steps/11-prepare-sqlx.sh` |
| Lint & format | `./steps/10-lint.sh` |
| Security audit | `./steps/03-audit.sh` |
| Build backend | `./steps/12-backend-cargo-build.sh` (add `--release` for release) |
| Build frontend | `./steps/17-frontend-npm-build.sh` |
| Generate TS bindings | `./steps/15-frontend-generate-bindings.sh` |
| Frontend dev server | `./steps/16-frontend-npm-dev.sh` |
| Bridge dev | `./steps/20-bridge-frontend-tauri-dev.sh` |
| Global manager | `./steps/13-run-pluralsync-global-manager.sh` |
| Rust unit tests | `./test/rust-tests.sh` |
| Integration tests | `./test/*.integration-tests.sh` (see `test/` for options) |
| Release build | `./steps/30-build-release.sh` |
| Update deps | `./steps/92-update-dependencies.sh` |

---

## Docker / Local Deployment

`docker/docker.compose.yml` defines 4 services: Nginx (frontend + reverse proxy), Rust backend, PostgreSQL, global manager.

Use `docker/local.env` or `docker/local-release.env` for environment variables. Nginx proxies `/api/` and `/fronting/` to the backend, serves frontend static files, and handles WebSocket upgrades for the bridge endpoint.

---

## Testing

**Always use scripts in `./test/`.** They handle setup, teardown, env vars, and error checking.

Run all relevant integration tests after making changes to sync logic, API endpoints, or configuration.

Don't use `docker logs` to access the (now stopped) containers after a test. Read the logs in `docker/logs/*` instead to debug system behavior.

---

## Script Numbering Conventions

| Range | Purpose |
|---|---|
| `01-*` | Cleanup |
| `02-*` | Dependency installation |
| `03-*` | Security audits |
| `10-*` | Linting / formatting |
| `11-*` | Database preparation |
| `12-*` | Backend builds |
| `13-*` | Running services |
| `14-*`–`17-*` | Frontend (announcements, bindings, dev, build) |
| `20-*`–`22-*` | Tauri/bridge (dev, build, release) |
| `30-*`–`33-*` | Release operations |
| `40-*`–`41-*` | Production monitoring |
| `90-*`–`92-*` | Analysis and maintenance |
