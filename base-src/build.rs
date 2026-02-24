fn main() {
    let version = extract_version_from_git();
    println!("cargo:rustc-env=PLURALSYNC_VERSION={version}");
}

fn extract_version_from_git() -> String {
    // Exact tag match (e.g., v2.59)
    if let Ok(output) = std::process::Command::new("git")
        .args(["describe", "--tags", "--exact-match"])
        .output()
        && output.status.success() {
            let tag = String::from_utf8_lossy(&output.stdout).into_owned();
            return normalize_tag(tag.trim());
        }

    // No tag - dev build from latest release
    if let Ok(output) = std::process::Command::new("git")
        .args(["describe", "--tags", "--abbrev=0"])
        .output()
        && output.status.success() {
            let tag = String::from_utf8_lossy(&output.stdout).into_owned();
            let base = normalize_tag(tag.trim());
            return format!("{}-dev", base.split('-').next().unwrap_or("0.1.0"));
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
