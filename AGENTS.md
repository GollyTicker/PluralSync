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

## Synchronization Architecture

### Data Source: SimplyPlural
SimplyPlural is the primary data source for all fronting status information:
*   Real-time WebSocket connection monitors fronting changes
*   HTTP API fetches system members, custom fronts, and privacy bucket configuration
*   User provides read-only token for secure API access
*   Includes members (archived/active), custom fronts, privacy settings

### Synchronization Targets (Updaters)
PluralSync synchronizes fronting status from SimplyPlural to multiple platforms:

**1. VRChat Status Updater** (server-managed)
*   Updates user's VRChat bio with current fronter name(s)
*   Requires VRChat credentials and authentication cookie
*   Integrates with custom "VRChat Status Name" field in SimplyPlural
*   Handles special character cleaning and rate-limit management

**2. Discord Rich Presence Updater** (server-managed or bridge-managed)
*   **Server variant**: Sends fronting data for Discord Rich Presence display
    *   Requires Discord bot token + user authorization
    *   Configurable display formats (short/long, with/without avatars)
    *   Supports activity types (Playing, Watching, Streaming, etc.)
*   **Status Message variant**: Updates Discord custom status (optional, per-deployment)
    *   Requires Discord self-bot token
*   **Desktop Bridge variant**: Local client management via Tauri desktop app
    *   WebSocket connection receives real-time updates
    *   Discord IPC loop runs continuously for immediate sync
    *   Independent of browser/web app (runs on system boot)

**3. PluralKit Fronters Sync Updater** (server-managed, to-pluralkit)
*   Synchronizes fronters from SimplyPlural to PluralKit system
*   Performs fronter switches via PluralKit API
*   Monitors rate-limit quota for API calls

**4. Website Fronting Display** (read-only)
*   Public API endpoint: `GET /fronting/<website_url_name>`
*   Returns formatted fronting status (JSON or HTML)
*   No authentication required
*   Respects privacy configuration and cleaning rules

### Update Flow
1. **Detection**: SimplyPlural fronting change detected via WebSocket or polling
2. **Processing**: Privacy rules applied, names cleaned per platform, formatted with limits
3. **Distribution**: Each enabled updater runs async to push changes to target platform

### Bridge (Desktop Application)
**pluralsync-bridge** is a Tauri-based desktop application:
*   **Backend** (`bridge-src-tauri`): Rust system integration (auto-start on boot)
*   **Frontend** (`bridge-frontend`): TypeScript/Vue login and settings UI
*   **Primary function**: Run Discord Rich Presence locally on user's machine
*   **WebSocket**: Real-time subscription to backend for fronting updates
*   **Advantage**: Keeps Discord RPC updated even when web app is closed
*   **Patches**: Custom patches applied to `discord-rich-presence` crate for compatibility

### Global Manager
**pluralsync-global-manager** binary:
*   Consumes SimplyPlural webhook events system-wide
*   Detects fronting changes and distributes to all connected updaters
*   Central event processing pipeline

### Configuration
User config enables/disables updaters:
*   `enable_vrchat` - VRChat bio updates
*   `enable_discord` - Discord Rich Presence
*   `enable_discord_status_message` - Discord status (if deployment-enabled)
*   `enable_to_pluralkit` - PluralKit synchronization

Server filters "foreign_managed" updaters (Discord bot management is user-controlled, not server-managed).

### Error Handling
*   Each updater tracks `last_operation_error`
*   Status enum: `Running`, `Disabled`, `Error(message)`, `Starting`
*   Metrics track API requests, rate limits, and failures per updater
*   All state changes and errors logged for debugging

---

## User Account Management

PluralSync includes comprehensive account management features built on email verification, secure token handling, and SMTP-based communication (using Brevo email service).

### Email Verification (Registration)
*   New users register via `POST /api/user/register` → creates temporary entry
*   `EmailVerificationToken` generated and hashed with app secret salt; 1-hour expiration
*   Verification email sent with link: `/verify-email?token={TOKEN}`
*   `POST /api/user/email/verify/<token>` endpoint converts temporary entry to permanent account
*   **Dev Mode:** Tokens printed to logs instead of being sent via email

### Email Address Change
*   Authenticated users request change via `POST /api/user/email/change`
*   System prevents duplicate emails (409 Conflict response)
*   Stores `new_email`, `email_verification_token_hash`, and `email_verification_token_expires_at` in user record
*   Two emails sent: confirmation link to NEW email, security notification to OLD email
*   Same `POST /api/user/email/verify/<token>` endpoint finalizes change after verification click
*   User record updated with new email; temporary fields cleared

### Password Reset
*   User requests reset via `POST /api/user/forgot-password` (email only; no auth required)
*   Always returns 200 OK to prevent email enumeration attacks
*   `PasswordResetToken` generated, hashed WITHOUT app secret, stored in `password_reset_requests` table with 1-hour expiration
*   Reset email sent with link: `/reset-password?token={TOKEN}`
*   User calls `POST /api/user/reset-password` with token and new password:
    *   System hashes token and verifies against stored request
    *   Updates user's password (hashed with unique salt)
    *   Deletes password reset request entry
    *   Records success/failure metrics

### Account Deletion
*   Authenticated users only: `DELETE /api/user` (requires JWT)
*   Security: requires password verification (prevents deletion via stolen JWT) + exact confirmation string: "delete"
*   Process:
    *   Verifies JWT and password
    *   Stops all updater tasks (syncing stops; logged if fails)
    *   Cascading database delete removes all related data
*   Frontend: `DeleteAccount.vue` component displays irreversible action warnings
*   Success: JWT cleared from localStorage, redirects to home page

### Security Features
*   Cryptographically secure token generation
*   PBKDF2 token hashing (app-secret salt for email tokens; no app-secret for password tokens)
*   1-hour token expiration on all time-sensitive operations
*   Password-protected account deletion
*   Async email delivery (non-blocking)
*   Email enumeration protection (forgot-password always returns success)

## Coding Guidelines

*   Rust import statemnts should be one crate per statement. Importing multiple objects from the same create should be done in the same statement.
    *   Good: `use anyhow::{anyhow, Error, Result}`
    *   Bad: the above imports on separate lines/statements for each imported object
*   Rust import statements should use separate lines for imports from different modules originating from this project.
*   In general, avoid excessive Debug and fmt::Display traits. Only add them, if printing to logs if useful. And then prefer Display over Debug.
*   **Security: Never add `#[derive(Debug)]` or `#[derive(Display)]` to structs containing sensitive fields** (passwords, tokens, API keys, cookies, secrets). If debugging output is needed, implement `fmt::Display` or `fmt::Debug` manually and redact all sensitive values (e.g., show only first 5 chars or `<field_name>`).
    *   Affected structs include: `UserConfigDbEntries`, `VRChatCredentials`, `VRChatCredentialsWithCookie`, `VRChatCredentialsWithTwoFactorAuth`, `UserLoginCredentials`, `SmtpConfig`, `ApplicationConfig`, and any struct with fields named `password`, `token`, `secret`, `cookie`, `api_key`, `credentials`. Check them for examples

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

#### Building Web Frontend
To generate TypeScript bindings from the Rust backend and build the web frontend:
```bash
./steps/15-frontend-generate-bindings.sh # Generate type-safe bindings
./steps/17-frontend-npm-build.sh # Build production frontend
```

#### Building Tauri Desktop Application
The desktop bridge application uses the Tauri framework and combines Rust backend logic with a web-based frontend:
```bash
./steps/20-bridge-frontend-tauri-dev.sh # Development build with hot reload
./steps/21-bridge-frontend-tauri-build.sh # Debug build (unbundled)
./steps/22-bridge-frontend-tauri-release.sh # Release build
```

#### Building Release Artifacts
To build release versions for multiple platforms:
```bash
./steps/30-build-release.sh # Builds Windows and Linux release artifacts
```

### Running (Local Development)
For local development, environment variables can be configured using `docker/local.env` as a template. The full-stack local execution involves the Docker setup defined in `docker/docker.compose.yml`.

#### Running the Web Frontend
```bash
./steps/16-frontend-npm-dev.sh # Starts the development server
```

#### Running the Desktop Application
```bash
./steps/20-bridge-frontend-tauri-dev.sh # Runs with hot reload
```
Ensure the backend is running separately if testing against it.

#### Running the Global Manager
The global manager handles SimplyPlural event processing and syncing across all connected users:
```bash
./steps/13-run-pluralsync-global-manager.sh # Requires GLOBAL_PLURALSYNC_SIMPLY_PLURAL_READ_WRITE_ADMIN_TOKEN
```

#### Starting Local Release Build
To test a complete local release build:
```bash
./steps/31-start-local-release.sh # Starts the backend and serves the frontend
```

### Database Preparation
Before running the backend, initialize the database with migrations:
```bash
./steps/11-prepare-sqlx.sh # Starts PostgreSQL container and runs migrations
```

### Testing

#### Unit Tests
To run Rust unit tests:
```bash
./test/unit-tests.sh
```
This executes `cargo test` for both the `base-src` and the root Rust projects.

#### Integration Tests
Multiple integration tests are available for different components:
*   **Manager Integration Tests:** `./test/manager.integration-tests.sh` - Tests the global manager's synchronization logic (requires `SPS_API_TOKEN`, `SPS_API_WRITE_TOKEN`, and optionally `PLURAL_SYSTEM_MEMBER_TO_TEST`)
*   **VRChat Integration Tests:** `./test/vrchat.integration-tests.sh` - Tests VRChat status synchronization (requires `VRCHAT_USERNAME`, `VRCHAT_PASSWORD`, and `VRCHAT_COOKIE`)
*   **Web Frontend Integration Tests:** `./test/frontend.needs-backend.integration-tests.sh` - Requires backend running separately
*   **Bridge Frontend Integration Tests:** `./test/bridge.needs-backend.integration-tests.sh` - Requires bridge backend built and backend running
*   **Webserver Integration Tests:** `./test/webserver.integration-tests.sh` - Tests website sync functionality
*   **Updater Integration Tests:** `./test/updater.integration-tests.sh` - Tests the update mechanism
*   **Restarts Integration Tests:** `./test/restarts.integration-tests.sh` - Tests behavior during restarts

#### End-to-End (e2e) Tests
*   **Web Frontend:** Refer to the `e2e` script in `frontend/package.json`. This typically requires the backend to be running separately.
*   **Bridge Frontend:** Refer to the `e2e` script in `bridge-frontend/package.json`. This requires `bridge-src-tauri` to be built and the backend to be running.

### Linting and Formatting
To lint and format the codebase:
```bash
./steps/10-lint.sh
```
This script runs `cargo clippy` with strict warning levels (`-W clippy::pedantic`, `-W clippy::nursery`, `-W clippy::unwrap_used`, `-W clippy::expect_used`), `rustfmt --edition 2024` for Rust code, and `prettier` for frontend code formatting. (Note: `eslint` linting for frontends is a pending TODO in the `web_frontend_lint` function within this script).

### Security Audits
To check for known security vulnerabilities in dependencies:
```bash
./steps/03-audit.sh
```
This runs `cargo audit` for all Rust crates (`base-src`, main backend, `bridge-src-tauri`) and `npm audit` for Node.js dependencies.

### Release Process
The release process is managed by creating a new Git tag and running:
```bash
./steps/32-publish-release.sh
```
This script verifies the current Git revision has a tag (e.g., `v2.10`), ensures GitHub CLI authentication, and publishes the release.

## Utility Scripts

### Cleanup
To clean build artifacts and dependencies:
```bash
./steps/01-clean.sh # Removes node_modules, target directories, and cleans all crates
```

### Dependency Management
To update all dependencies to their latest versions:
```bash
./steps/92-update-dependencies.sh # Updates Cargo and npm packages
```

### Code Analysis
*   **Codebase Size Analysis:** `./steps/90-codebase-size.sh` - Uses `cloc` to analyze lines of code and shows largest Rust files
*   **Build Time Analysis:** `./steps/91-build-time-analysis.sh` - Generates detailed build timing reports for all components

### Event Processing (Production Monitoring)
For troubleshooting and monitoring production instances:
*   `./steps/40-get-sp-events.sh` - Retrieves SimplyPlural webhook events from production logs (SSH access required)
*   `./steps/41-process-sp-events.sh` - Processes and parses SP events into structured JSON format

## Binary Targets

The project includes multiple Rust binary targets in the main crate:

*   **pluralsync:** The main API server and WebSocket backend
*   **ts-bindings:** Generates TypeScript type definitions from Rust using `specta`, ensuring type safety between backend and frontend
*   **pluralsync-global-manager:** Handles global event processing and synchronization across all users' plural systems

## Patches

The project includes patches for external dependencies:
*   `discord-rich-presence.activity.rs.patch` - Patch for the Discord Rich Presence activity integration
*   `discord-rich-presence.discord_ipc.rs.patch` - Patch for Discord IPC communication

These patches are applied automatically during dependency installation via `./steps/02-install-dependencies.sh`.
