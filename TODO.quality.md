# PluralSync Code Quality & Refactoring TODO

## 📋 High Priority Tasks

### 1. Split Config.vue (876 lines → 5 components)
- **Issue**: Single mega-component handling all settings
- **Solution**: Break into:
  - VRChatConfigPanel.vue
  - DiscordConfigPanel.vue
  - PluralKitConfigPanel.vue
  - WebsiteConfigPanel.vue
  - SimplyPluralConfigPanel.vue
- **Impact**: Better maintainability, clearer separation of concerns

### 2. Backend Module Re-exports Standardization
- **Issue**: Blanket `pub use *` exports make public API unclear
- **Files**: 8 mod.rs files (users/, plurality/, updater/, database/, platforms/)
- **Solution**: Replace with explicit re-exports listing only intended public items
- **Impact**: Clearer public APIs, better code navigation

### 3. Split Large Monolithic Files

#### queries.rs (575 lines → 5 files)
- **Functions**: 23 public async functions
- **Split by domain**:
  - user_queries.rs: User/auth related queries
  - updater_queries.rs: Updater state queries
  - config_queries.rs: Configuration queries
  - website_queries.rs: Website sync queries
- **Target size**: ~150-200 lines per file

#### user_api.rs (526 lines → 3-4 files)
- **Current**: All endpoints mixed together
- **Split by concern**:
  - auth_endpoints.rs: register, login, forgot-password, reset-password, verify-email
  - user_endpoints.rs: get info, delete account, change email
  - config_endpoints.rs: get/set config, defaults

#### manager.rs (536 lines)
- **Review**: Complex updater state machine logic
- **Consider splitting**:
  - state.rs: State definitions and transitions
  - lifecycle.rs: Startup/shutdown/restart logic
  - operations.rs: Update operations and error handling

---

## 📊 Medium Priority Tasks

### 4. API Client Organization
- **File**: frontend/src/pluralsync_api.ts (148 lines)
- **Split into**:
  - auth_api.ts: Login, register, password operations
  - config_api.ts: Get/set config, defaults
  - updaters_api.ts: Updater status and control
  - platform_api.ts: VRChat, Discord, PluralKit specific endpoints
- **Benefit**: Clearer API surface, easier navigation

### 5. Type Safety Cleanup
- **Scope**: All 42 Rust files
- **Goal**: Enforce AGENTS.md import guidelines:
  - One crate per statement (mostly done)
  - Separate project imports by module
- **Check**: `grep -r "^use" src/ --include="*.rs"` for patterns

### 6. Import Statement Consistency
- **Backend**: Ensure all files follow pattern:
  ```rust
  use anyhow::{anyhow, Result};
  use serde::Serialize;
  use crate::database;
  use crate::users::{self, UserConfig};
  ```
- **Frontend**: Already completed

---

## 🔧 Low Priority Tasks

### 7. Code Duplication & Patterns
- **Scope**: 15 files with potential duplication
- **Areas to review**:
  - Error handling patterns
  - Database query result processing
  - API request/response handling
  - Configuration loading and validation

### 8. Set Up ESLint for Bridge Frontend
- **Status**: ESLint not installed
- **Action**: Install eslint packages, configure rules
- **Goal**: Consistency with main frontend

### 9. Updater State Machine Review
- **File**: src/updater/manager.rs
- **Goal**: Improve clarity of state transitions
- **Consider**: Documentation or refactoring of complex logic

---

## 📈 Execution Strategy

**Phase 1: Module Organization**
- Module re-exports standardization (4-5 hours)
- Import consistency enforcement (2-3 hours)

**Phase 2: File Splitting**
- queries.rs split (3-4 hours)
- user_api.rs reorganization (2-3 hours)
- Testing between splits (2-3 hours)

**Phase 3: Frontend Improvements**
- Config.vue split (4-5 hours)
- Component rename (1-2 hours)
- API client reorganization (2-3 hours)

**Phase 4: Polish**
- Pattern extraction (as needed)
- Bridge frontend linting (1-2 hours)
- Final validation

---

## 🎯 Quality Goals

- Zero clippy warnings (current: ✅ already achieved)
- Zero frontend linting errors (current: ✅ fixed)
- All files <400 lines where practical
- Clear module boundaries and public APIs
- Consistent import patterns throughout
