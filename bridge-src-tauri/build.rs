use std::fs;

const CONFIG_TEMPLATE: &str = "template.tauri.conf.json";
const CONFIG: &str = "tauri.conf.json";

#[allow(clippy::unwrap_used)]
fn main() {
    let version = fs::read_to_string("../target/version.txt").unwrap();
    println!("cargo:rustc-env=PLURALSYNC_VERSION={version}");

    let public_key = std::env::var("TAURI_SIGNING_PUBLIC_KEY").unwrap();

    // Inject version and public key into tauri.conf.json
    let mut config: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(CONFIG_TEMPLATE).unwrap()).unwrap();

    config["version"] = serde_json::json!(version);
    config["plugins"]["updater"]["pubkey"] = serde_json::json!(public_key);

    fs::write(CONFIG, serde_json::to_string_pretty(&config).unwrap()).unwrap();

    tauri_build::build();
}
