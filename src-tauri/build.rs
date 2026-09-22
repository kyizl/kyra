fn main() {
    let update_channel =
        std::env::var("KYRA_UPDATE_CHANNEL").unwrap_or_else(|_| "production".to_owned());
    let build_commit = std::env::var("KYRA_BUILD_COMMIT").unwrap_or_default();
    println!("cargo:rerun-if-env-changed=KYRA_UPDATE_CHANNEL");
    println!("cargo:rerun-if-env-changed=KYRA_BUILD_COMMIT");
    println!("cargo:rustc-env=KYRA_UPDATE_CHANNEL={update_channel}");
    println!("cargo:rustc-env=KYRA_BUILD_COMMIT={build_commit}");
    tauri_build::build()
}
