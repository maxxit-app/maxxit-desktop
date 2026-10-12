#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
fn main() {
    if std::env::args().any(|arg| arg == "--version") {
        println!(
            "maxxit {} {}",
            env!("CARGO_PKG_VERSION"),
            env!("MAXXIT_SOURCE_COMMIT")
        );
        return;
    }
    #[cfg(debug_assertions)]
    {
        let args: Vec<String> = std::env::args().collect();
        if let Some(index) = args.iter().position(|v| v == "--diagnostics-probe") {
            let mode = args.get(index + 1).expect("Probe mode");
            let dir = args.get(index + 2).expect("Isolated probe directory");
            maxxit_lib::observability::probe(dir.into(), mode);
            return;
        }
    }
    let reporter =
        sentry::integrations::minidump::MinidumpIntegration::new().is_crash_reporter_process();
    if let Some(directory) = maxxit_lib::observability::data_directory() {
        maxxit_lib::observability::initialize(directory, reporter);
    }
    if reporter {
        return;
    }
    if std::env::args().any(|arg| arg == "--capture-claude") {
        let _ = maxxit_lib::observability::observe(
            maxxit_lib::observability::Operation::new("bridge_capture", "bridge", None),
            maxxit_lib::bridge::capture,
        );
        return;
    }
    maxxit_lib::run();
}
