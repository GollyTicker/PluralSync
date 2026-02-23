# Autoupdating Bridge - Simplified Specification

## Core Requirements

1. **Auto-update support**: Windows (NSIS) + Linux (AppImage)
2. **Manual update notification**: deb/rpm users (blocked until updated)
3. **Version from git tag**: Always derive from tag, no manual updates
4. **Immediate enforcement**: No grace period for old clients
5. **Signing key**: From `SIGNING_KEY` environment variable

---

## Key Assumptions Verified

| Assumption | Status | Notes |
|------------|--------|-------|
| Current latest tag | ✅ `v2.59` | Pre-release tags exist (e.g., `v2.53-rc4`) |
| `base-src` already uses `GIT_TAG` env | ✅ | `base-src/build.rs` reads `GIT_TAG` |
| `PLURALSYNC_VERSION` usage | ✅ | Used in bridge + backend via `base-src/src/meta.rs` |
| Tauri v2 config | ✅ `tauri.conf.json` | Version is static JSON, needs build.rs modification |
| No workspace setup | ✅ | Root `Cargo.toml` has no `[workspace]` section |

---

## Simplified Implementation Plan

### Phase 1: Version from Git Tag (2 files changed)

**Goal**: `v2.59` → `2.59.0`, no tag → `2.59.0-dev`

#### 1. Update `base-src/build.rs`

Current:
```rust
fn main() {
    let version = std::env::var("GIT_TAG").unwrap_or_else(|_| "dev".to_string());
    println!("cargo:rustc-env=PLURALSYNC_VERSION={version}");
}
```

Replace with:
```rust
fn main() {
    let version = extract_version_from_git();
    println!("cargo:rustc-env=PLURALSYNC_VERSION={}", version);
}

fn extract_version_from_git() -> String {
    // Exact tag match (e.g., v2.59)
    if let Ok(output) = std::process::Command::new("git")
        .args(["describe", "--tags", "--exact-match"])
        .output()
    {
        if output.status.success() {
            let tag = String::from_utf8_lossy(&output.stdout).trim();
            return normalize_tag(tag);
        }
    }
    
    // No tag - dev build from latest release
    if let Ok(output) = std::process::Command::new("git")
        .args(["describe", "--tags", "--abbrev=0"])
        .output()
    {
        if output.status.success() {
            let tag = String::from_utf8_lossy(&output.stdout).trim();
            let base = normalize_tag(tag);
            return format!("{}-dev", base.split('-').next().unwrap());
        }
    }
    
    "0.1.0-dev".to_string()
}

fn normalize_tag(tag: &str) -> String {
    let tag = tag.strip_prefix('v').unwrap_or(tag);
    let parts: Vec<&str> = tag.split('-').collect();
    let main = parts[0];
    
    // Ensure 3 components: 2.59 → 2.59.0
    let mut main_parts: Vec<&str> = main.split('.').collect();
    while main_parts.len() < 3 {
        main_parts.push("0");
    }
    
    let normalized = main_parts.join(".");
    if parts.len() > 1 {
        format!("{}-{}", normalized, parts[1..].join("-"))
    } else {
        normalized
    }
}
```

#### 2. Update `bridge-src-tauri/build.rs`

Add version injection for `tauri.conf.json`:

```rust
use std::fs;

fn main() {
    tauri_build::build();
    
    // Inject version into tauri.conf.json
    let version = std::env::var("PLURALSYNC_VERSION")
        .unwrap_or_else(|_| "0.1.0".to_string());
    
    let config_path = "tauri.conf.json";
    let mut config: serde_json::Value = 
        serde_json::from_str(&fs::read_to_string(config_path).unwrap()).unwrap();
    
    config["version"] = serde_json::json!(version);
    fs::write(config_path, serde_json::to_string_pretty(&config).unwrap()).unwrap();
}
```

Add dependency to `bridge-src-tauri/Cargo.toml`:
```toml
[build-dependencies]
tauri-build = "*"
serde_json = "*"
```

---

### Phase 2: Tauri Updater (4 files changed)

#### 2.1 Generate Signing Key (one-time)

```bash
cargo tauri signer generate
# Store output in SIGNING_KEY env var
```

#### 2.2 `bridge-src-tauri/Cargo.toml`

Add:
```toml
[dependencies]
tauri-plugin-updater = "2"
```

#### 2.3 `bridge-frontend/package.json`

Add:
```json
{
  "dependencies": {
    "@tauri-apps/plugin-updater": "^2.0.0"
  }
}
```

#### 2.4 `bridge-src-tauri/tauri.conf.json`

Add to end of file (before closing `}`):
```json
,
"plugins": {
  "updater": {
    "active": true,
    "dialog": false,
    "endpoints": [
      "https://github.com/GollyTicker/PluralSync/releases/latest/download/latest.json"
    ],
    "pubkey": "<PUBLIC_KEY_HERE>"
  }
}
```

#### 2.5 `bridge-src-tauri/src/lib.rs`

Add updater integration:

```rust
// After line 188 (before tauri::Builder):
let updater_plugin = tauri_plugin_updater::Builder::new().build();

// In .setup() chain, add:
.plugin(updater_plugin)
```

---

### Phase 3: Backend Version Check (3 files changed)

#### 3.1 `base-src/src/users.rs`

Add to `UserLoginCredentials`:
```rust
pub client_version: Option<String>,
```

#### 3.2 `src/users/auth_endpoints.rs`

Add to `post_api_user_login` (after logging, before user lookup):

```rust
// Version check - immediate enforcement
if let Some(ref client_ver) = credentials.client_version {
    if !is_version_acceptable(client_ver) {
        return Err((
            http::Status::UpgradeRequired,
            format!("Client {} is outdated. Update required.", client_ver)
        ));
    }
}

fn is_version_acceptable(client: &str) -> bool {
    use pluralsync_base::meta::MINIMUM_BRIDGE_VERSION;
    // Simple string comparison for now
    client >= MINIMUM_BRIDGE_VERSION
}
```

#### 3.3 `base-src/src/meta.rs`

Add constant:
```rust
pub const MINIMUM_BRIDGE_VERSION: &str = "2.59.0";  // Update per release
```

---

### Phase 4: Frontend Update UI (3 files changed)

#### 4.1 `bridge-frontend/src/pages/login-page.ts`

Add version to login:

```typescript
import { invoke } from '@tauri-apps/api/core'

// In login handler:
const version = await invoke<string>('get_bridge_version')
let creds: UserLoginCredentials = {
  email: { inner: email },
  password: { inner: { inner: password } },
  client_version: version  // NEW
}
```

#### 4.2 `bridge-frontend/src/pages/status-page.ts`

Add update check:

```typescript
import { check } from '@tauri-apps/plugin-updater'
import { invoke } from '@tauri-apps/api/core'

async function renderStatusPage() {
  const version = await invoke<string>('get_bridge_version')
  
  document.querySelector<HTMLDivElement>('#app')!.innerHTML = `
    <div>
      <h1>PluralSync Bridge</h1>
      <div>Version: ${version}</div>
      <button id="check-updates">Check for Updates</button>
      <div id="updater-status"></div>
      <!-- existing status content -->
    </div>
  `
  
  document.querySelector('#check-updates')?.addEventListener('click', async () => {
    try {
      const update = await check()
      if (update) {
        if (confirm(`Update ${update.version} available. Install now?`)) {
          await update.downloadAndInstall()
        }
      } else {
        document.querySelector('#updater-status')!.textContent = 'Up to date'
      }
    } catch (e) {
      console.error(e)
      document.querySelector('#updater-status')!.textContent = 'Update check failed'
    }
  })
}
```

---

### Phase 5: Release Script (2 files changed)

#### 5.1 `steps/32-publish-release.sh`

Add signing after build:

```bash
# After ./steps/30-build-release.sh

OUT_DIR="target/release_builds"

# Sign artifacts
if [ -n "${SIGNING_KEY:-}" ]; then
  if [ -f "$OUT_DIR/PluralSync-Bridge-Windows-Setup.exe" ]; then
    cargo tauri signer sign --key "$SIGNING_KEY" \
      "$OUT_DIR/PluralSync-Bridge-Windows-Setup.exe"
  fi
  
  if [ -f "$OUT_DIR/PluralSync-Bridge-Linux.AppImage" ]; then
    cargo tauri signer sign --key "$SIGNING_KEY" \
      "$OUT_DIR/PluralSync-Bridge-Linux.AppImage"
  fi
else
  echo "Warning: SIGNING_KEY not set, artifacts not signed"
fi

# Generate latest.json
cat > "$OUT_DIR/latest.json" << EOF
{
  "version": "$VERSION",
  "notes": "Release $TAG",
  "pub_date": "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
}
EOF
```

#### 5.2 `steps/30-build-release.sh`

No changes needed - version comes from git tag via `build.rs`

---

## File Change Summary

| Phase | Files Changed | Lines of Code |
|-------|---------------|---------------|
| 1. Version | `base-src/build.rs`, `bridge-src-tauri/build.rs` | ~60 |
| 2. Updater | `Cargo.toml`, `package.json`, `tauri.conf.json`, `lib.rs` | ~20 |
| 3. Backend | `users.rs`, `auth_endpoints.rs`, `meta.rs` | ~30 |
| 4. Frontend | `login-page.ts`, `status-page.ts` | ~40 |
| 5. Release | `32-publish-release.sh` | ~30 |
| **Total** | **9 files** | **~180 lines** |

---

## Testing Checklist

- [ ] Dev build: no tag → version shows `2.59.0-dev`
- [ ] Tagged build: `v2.59` → version shows `2.59.0`
- [ ] Pre-release: `v2.60-rc2` → version shows `2.60.0-rc2`
- [ ] Windows auto-update works
- [ ] Linux AppImage auto-update works
- [ ] Old client blocked on login (426 response)
- [ ] Update notification shows in UI
- [ ] Release script signs artifacts

---

## APT/YUM Repository (Future)

**Decision**: Not implementing now. Users manage deb/rpm manually.

**If needed later**: GitHub Pages + reprepro (~4-8 hours setup)
- Free hosting
- Automatic `apt update && apt upgrade` for users
- Requires separate repo + GPG key management

---

## Email Notifications (Deferred)

1. **Pre-announcement**: "Hello" + auto-update teaser
2. **Launch email**: Auto-update feature announcement

Send after implementation complete.
