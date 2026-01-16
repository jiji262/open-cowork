// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod config;
mod events;
mod providers;
mod state;
mod tools;

#[tauri::command]
fn ping_cmd() -> &'static str {
  app_lib::ping()
}

fn main() {
  tauri::Builder::default()
    .manage(state::SessionState::new())
    .plugin(tauri_plugin_dialog::init())
    .invoke_handler(tauri::generate_handler![
      ping_cmd,
      commands::session::session_list,
      commands::session::recent_cwds,
      commands::client_event::client_event
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
