#[allow(clippy::unwrap_used)]
fn main() {
    let version = std::env::var("VERSION").unwrap();
    println!("cargo:rustc-env=PLURALSYNC_VERSION={version}");

    tauri_build::build();
}
