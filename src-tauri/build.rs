fn main() {
    println!("cargo:rerun-if-env-changed=MAXXIT_SOURCE_COMMIT");
    let sha = std::env::var("MAXXIT_SOURCE_COMMIT")
        .ok()
        .or_else(|| {
            std::process::Command::new("git")
                .args(["rev-parse", "HEAD"])
                .output()
                .ok()
                .filter(|output| output.status.success())
                .and_then(|output| String::from_utf8(output.stdout).ok())
        })
        .unwrap_or_else(|| "unavailable".into());
    let sha = sha.trim();
    assert!(
        sha == "unavailable" || (sha.len() == 40 && sha.chars().all(|c| c.is_ascii_hexdigit())),
        "Invalid source revision"
    );
    println!("cargo:rustc-env=MAXXIT_SOURCE_COMMIT={sha}");
    let branch = std::process::Command::new("git")
        .args(["symbolic-ref", "-q", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .unwrap_or_else(|| "HEAD".into());
    for name in ["HEAD", branch.as_str(), "packed-refs"] {
        if let Ok(output) = std::process::Command::new("git")
            .args(["rev-parse", "--git-path", name])
            .output()
        {
            if output.status.success() {
                println!(
                    "cargo:rerun-if-changed={}",
                    String::from_utf8_lossy(&output.stdout).trim()
                );
            }
        }
    }
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "snapshot",
            "export_token_csv",
            "tray_action",
            "tray_resize",
            "save_settings",
            "claude_preview",
            "connect_provider",
            "disconnect_provider",
            "save_project",
            "remove_project",
            "cloud_start",
            "cloud_redeem",
            "cloud_disconnect",
            "cloud_sync",
            "cloud_action",
            "cloud_preferences",
            "open_link",
        ]),
    ))
    .expect("Tauri permission generation failed");
}
