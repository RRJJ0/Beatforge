// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if std::env::args().any(|argument| argument == "--mock-worker") {
        std::process::exit(app_lib::run_mock_worker());
    }
    app_lib::run();
}
