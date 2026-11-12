// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use nodaris_core::error::CoreError;
use nodaris_core::models::AccessPoint;

// Команда, которую мы будем вызывать из Svelte
#[tauri::command]
async fn scan_wifi() -> Result<Vec<AccessPoint>, CoreError> {
    // Вызываем наш кросс-платформенный (пока Linux) крейт напрямую
    nodaris_linux::scan_wifi()
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![scan_wifi])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
