# Instructions for AI Coding Agents

## What is PluralSync?

A cloud service where users can automatically sync their plural systems' fronting status
between various system managers and social platforms such as [SimplyPLURAL](https://apparyllis.com/), [PluralKit](https://pluralkit.me/), [VRChat](https://hello.vrchat.com/), [Discord](https://discord.com) or their own website. Users of system managers (plural systems, DID/OSDD systems, etc.) benefit from this as it makes it easier for them to communicate who's fronting while only
needing to update their fronting on Simply Plural.

A public test version can be found online at [public-test.pluralsync.ayake.net](https://public-test.pluralsync.ayake.net). (*Use this at your own risk.*)

Currently the following updates are supported:
* SimplyPlural to VRChat Status
* SimplyPlural to Discord Status / Discord Rich Presence
* SimplyPlural to Website with fronter names and avatars
* SimplyPlural to PluralKit Fronters Switch

We, the developers, take data security and privacy seriously. The data to synchronise between the services
is stored encrypted and at industry-standard security. Self hosting is possible if you have some tech knowledge.

## General DOs

Only do the tasks described when explicitly requested to.

## Project Overview
PluralSync is a cloud service designed to synchronize the "fronting" status of plural systems across various platforms. It is built as a multi-component application with a Rust backend, a main web frontend (Vue.js/Vite), and a desktop bridge frontend (Tauri/Vite). It uses PostgreSQL for data persistence and Docker/Nginx for deployment/local development. The project emphasizes data security and privacy.

## Architecture

### Backend (`pluralsync`)
*   **Language:** Rust
*   **Framework:** [Rocket](https://rocket.rs/)
*   **Key Technologies:** SQLx (PostgreSQL), Tokio (async runtime)
*   **Functionality:**
    *   Provides the core backend services, including a RESTful API and WebSocket communication.
    *   Interacts with a PostgreSQL database using `sqlx` for data persistence.
    *   Manages user authentication using JSON Web Tokens (JWT).
    *   Communicates with external services like the VRChat API.
    *   Exposes application metrics for monitoring via Prometheus.
*   **Tooling:**
    *   Includes a utility (`ts-bindings`) to generate TypeScript type definitions from Rust code using `specta`, ensuring type safety between the backend and frontend.

### Web Frontend (`frontend`)
*   **Framework:** [Vue.js](https://vuejs.org/) with TypeScript
*   **Build Tool:** [Vite](https://vitejs.dev/)
*   **Key Technologies:** Vue Router, Axios
*   **Functionality:**
    *   Provides the main user interface for the web application.
    *   Communicates with the Rust backend via HTTP requests (using `axios`) and WebSockets.

### Desktop Application (`pluralsync-bridge`)
*   **Framework:** [Tauri](https://tauri.app/) (Rust backend, web-based frontend)
*   **Backend (`bridge-src-tauri`):**
    *   Written in Rust.
    *   Integrates with the operating system for features like autostart.
    *   Includes Discord Rich Presence integration.
    *   Communicates with the main `pluralsync` backend.
*   **Frontend (`bridge-frontend`):**
    *   A web-based UI built with TypeScript and Vite.
    *   **Key Technologies:** Navigo (routing)
    *   Uses the Tauri API to interact with the Rust backend part of the desktop application.

### Shared Code (`base-src`)
*   **Language:** Rust
*   **Purpose:**
    *   A shared library containing common data structures, types, and utilities.
    *   This crate is used as a dependency by both the main backend (`pluralsync`) and the Tauri backend (`pluralsync-bridge`), promoting code reuse and consistency.

## Coding Guidelines

*   Rust import statemnts should be one crate per statement. Importing multiple objects from the same create should be done in the same statement.
    *   Good: `use anyhow::{anyhow, Error, Result}`
    *   Bad: the above imports on separate lines/statements for each imported object
*   Rust import statements should use separate lines for imports from different modules originating from this project.

## Development Workflows

### Prerequisites
*   Rust toolchain (installation via `rustup` is recommended)
*   Node.js (v20.19.0 or >=22.12.0) + npm

### Installation of Dependencies
To set up the development environment and install all necessary dependencies:
```bash
./steps/02-install-dependencies.sh
```
This script handles system-level packages (e.g., `oathtool`, `mingw-w64`, `libwebkit2gtk-4.1-dev`), installs Rust cargo tools (`cargo-audit`, `sqlx-cli`, `tauri-cli`, `tauri-driver`), applies patches to the `discord-rich-presence` crate, and installs Node.js dependencies for both `bridge-frontend` and `frontend` projects using `npm ci`.

### Building
#### Building the Backend
To build the Rust backend:
```bash
./steps/12-backend-cargo-build.sh # Builds in debug mode
./steps/12-backend-cargo-build.sh --release # Builds in release mode
```
#### Building Frontends
Frontends are typically built as part of their respective `npm run build` scripts or when building the Tauri application.

### Running (Local Development)
For local development, environment variables can be configured using `docker/local.env` as a template. The full-stack local execution likely involves the Docker setup defined in `docker/docker.compose.yml`. Specific instructions for starting the backend and frontends are often tied to the `dev` scripts in `package.json` files and integration test setups.

### Testing

#### Unit Tests
To run Rust unit tests:
```bash
./test/unit-tests.sh
```
This executes `cargo test` for both the `base-src` and the root Rust projects.

#### End-to-End (e2e) Tests
*   **Web Frontend:** Refer to the `e2e` script in `frontend/package.json`. This typically requires the backend to be running separately.
*   **Bridge Frontend:** Refer to the `e2e` script in `bridge-frontend/package.json`. This requires `bridge-src-tauri` to be built and the backend to be running.
*   Additional integration tests are located in the `test` directory (e.g., `test/manager.integration-tests.sh`, `test/vrchat.integration-tests.sh`). These often depend on specific environment variables (e.g., `SPS_API_TOKEN`, `VRCHAT_USERNAME`).

### Linting and Formatting
To lint and format the codebase:
```bash
./steps/10-lint.sh
```
This script runs `cargo clippy` with strict warning levels, `rustfmt` for Rust code, and `prettier` for frontend code formatting. (Note: `eslint` linting for frontends is a pending TODO in the `web_frontend_lint` function within this script).

### Release Process
The release process is managed by creating a new Git tag and running `./steps/32-publish-release.sh`.
