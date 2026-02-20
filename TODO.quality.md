# PluralSync Code Quality & Refactoring TODO

## 📋 High Priority Tasks

### 1. ✅ Split ConfigSettings.vue (878 lines → 7 components) [COMPLETE]
- **Issue**: Single mega-component handling all settings
- **Solution**: Split into 6 focused panels + lightweight parent:
  - ✅ SimplyPluralConfigPanel.vue (213 lines)
  - ✅ PluralKitConfigPanel.vue (62 lines)
  - ✅ WebsiteConfigPanel.vue (64 lines)
  - ✅ FrontingStatusTextPanel.vue (57 lines)
  - ✅ DiscordConfigPanel.vue (108 lines)
  - ✅ VRChatConfigPanel.vue (154 lines)
  - ✅ ConfigSettings.vue refactored (331 lines) - parent orchestrator
- **Result**: 
  - Each panel is self-contained with independent logic
  - Parent focuses only on account settings & form coordination
  - UI layout and styling preserved
  - All linting passes (ESLint ✅, TypeScript ✅, Prettier ✅)
  - Build succeeds with no errors
- **Impact**: Better maintainability, clearer separation of concerns, easier to test

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

### 4. API Client Organization [NEXT FRONTEND TASK]
- **File**: frontend/src/pluralsync_api.ts (148 lines)
- **Status**: Candidate for Phase 3 continuation
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

### ✅ Phase 3 (COMPLETE): Frontend Improvements
- ✅ ConfigSettings.vue split (4-5 hours)
- ⏭️ Component rename (1-2 hours) - *if needed for consistency*
- ⏭️ API client reorganization (2-3 hours) - *next task*

### ⏭️ Next Phases

**Phase 1: Module Organization** (Backend)
- Module re-exports standardization (4-5 hours)
- Import consistency enforcement (2-3 hours)

**Phase 2: File Splitting** (Backend)
- queries.rs split (3-4 hours)
- user_api.rs reorganization (2-3 hours)
- Testing between splits (2-3 hours)

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
