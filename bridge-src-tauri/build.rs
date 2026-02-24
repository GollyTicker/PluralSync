use std::fs;

#[allow(clippy::expect_used)]
fn main() {
    tauri_build::build();

    // Inject version and public key into tauri.conf.json
    let version = std::env::var("PLURALSYNC_VERSION")
        .unwrap_or_else(|_| "0.1.0".to_string());

    let public_key = std::env::var("TAURI_SIGNING_PUBLIC_KEY")
        .unwrap_or_else(|_| "<PUB_KEY_HERE>".to_string());

    let config_path = "tauri.conf.json";
    let mut config: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(config_path).expect("Failed to read tauri.conf.json"))
            .expect("Failed to parse tauri.conf.json");

    config["version"] = serde_json::json!(version);
    config["plugins"]["updater"]["pubkey"] = serde_json::json!(public_key);

    fs::write(config_path, serde_json::to_string_pretty(&config).expect("Failed to write tauri.conf.json"))
        .expect("Failed to write tauri.conf.json");
}
