#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }
      Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

pub fn ping() -> &'static str {
  "pong"
}

mod providers;
mod runtime;
mod storage;
mod tools;

#[cfg(test)]
mod tests {
  use super::ping;

  #[test]
  fn ping_returns_pong() {
    assert_eq!(ping(), "pong");
  }
}
