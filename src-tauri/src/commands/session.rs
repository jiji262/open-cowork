use tauri::{AppHandle, Emitter, State};

use crate::events::ServerEvent;
use crate::state::SessionState;

#[tauri::command]
pub fn session_list(app: AppHandle, state: State<SessionState>) -> Result<(), String> {
  let event = ServerEvent::SessionList {
    sessions: state.list_sessions(),
  };
  app.emit("server-event", event).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn recent_cwds(state: State<SessionState>, limit: Option<usize>) -> Vec<String> {
  let limit = limit.unwrap_or(8).clamp(1, 20);
  state.list_recent_cwds(limit)
}
