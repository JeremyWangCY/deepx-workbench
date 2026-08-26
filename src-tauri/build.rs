fn main() {
    let app_manifest = tauri_build::AppManifest::new().commands(&[
        "runtime_status",
        "update_status",
        "update_deepx",
        "initialize_harness",
        "launch_harness",
        "show_harness",
        "update_harness",
        "marketplace_status",
        "install_marketplace",
    ]);
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(app_manifest))
        .expect("failed to run tauri build");
}
