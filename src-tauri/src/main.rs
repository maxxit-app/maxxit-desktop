#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
fn main() {
    if std::env::args().any(|arg| arg == "--capture-claude") {
        if let Err(error) = maxxit_lib::bridge::capture() {
            eprintln!("Maxxit usage bridge: {error}");
        }
        return;
    }
    maxxit_lib::run();
}
