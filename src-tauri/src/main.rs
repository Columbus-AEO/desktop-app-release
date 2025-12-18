// Prevents additional console window on Windows in release builds only
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    columbus_desktop::run()
}
