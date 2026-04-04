use std::fs;

const CONFIG_TEMPLATE: &str = "template.tauri.conf.json";
const CONFIG: &str = "tauri.conf.json";

#[allow(clippy::unwrap_used)]
fn main() {
    let version = extract_version_from_git();
    println!("cargo:rustc-env=PLURALSYNC_VERSION={version}");

    // Inject version and public key into tauri.conf.json
    let public_key = std::env::var("TAURI_SIGNING_PUBLIC_KEY").unwrap();

    let mut config: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(CONFIG_TEMPLATE).unwrap()).unwrap();

    config["version"] = serde_json::json!(version);
    config["plugins"]["updater"]["pubkey"] = serde_json::json!(public_key);

    fs::write(CONFIG, serde_json::to_string_pretty(&config).unwrap()).unwrap();

    tauri_build::build();
}

// THIS CODE IS DUPLICATED IN MULTIPLE PLACES!!
#[allow(clippy::unwrap_used)]
fn extract_version_from_git() -> String {
    // Exact tag match (e.g., v2.59)
    if let Ok(output) = std::process::Command::new("git")
        .args(["describe", "--tags", "--exact-match"])
        .output()
        && output.status.success()
    {
        let tag = String::from_utf8_lossy(&output.stdout).into_owned();
        return normalize_tag(tag.trim());
    }

    // No tag - dev build from latest release
    let output = std::process::Command::new("git")
        .args(["describe", "--tags", "--abbrev=0"])
        .output()
        .unwrap();

    assert!(output.status.success());

    let tag = String::from_utf8_lossy(&output.stdout).into_owned();
    let base = normalize_tag(tag.trim());
    format!("{}-dev", base.split('-').next().unwrap())
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
