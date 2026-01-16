use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;
use tokio::sync::oneshot;

use crate::events::{PermissionMode, SessionInfo, SessionStatus};
use crate::providers::registry::ProviderKind;

#[derive(Clone)]
pub struct ProviderSettings {
  pub provider: ProviderKind,
  pub api_key: String,
  pub model: String,
  pub base_url: Option<String>,
  pub permission_mode: PermissionMode,
}

#[derive(Default)]
pub struct SessionState {
  sessions: Mutex<HashMap<String, SessionInfo>>,
  messages: Mutex<HashMap<String, Vec<Value>>>,
  providers: Mutex<HashMap<String, ProviderSettings>>,
  pending_permissions: Mutex<HashMap<String, oneshot::Sender<Value>>>,
}

impl SessionState {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn list_sessions(&self) -> Vec<SessionInfo> {
    let mut list: Vec<SessionInfo> = self
      .sessions
      .lock()
      .expect("session lock")
      .values()
      .cloned()
      .collect();
    list.sort_by_key(|session| session.updated_at);
    list
  }

  pub fn get_session(&self, id: &str) -> Option<SessionInfo> {
    self
      .sessions
      .lock()
      .expect("session lock")
      .get(id)
      .cloned()
  }

  pub fn get_messages(&self, id: &str) -> Vec<Value> {
    self
      .messages
      .lock()
      .expect("message lock")
      .get(id)
      .cloned()
      .unwrap_or_default()
  }

  pub fn create_session(
    &self,
    title: String,
    cwd: Option<String>,
    provider: ProviderSettings,
  ) -> SessionInfo {
    let now = now_ms();
    let id = format!("session-{}", now);
    let session = SessionInfo {
      id: id.clone(),
      title,
      status: SessionStatus::Running,
      cwd,
      claude_session_id: None,
      provider: Some(provider.provider.clone()),
      model: Some(provider.model.clone()),
      created_at: now,
      updated_at: now,
    };

    self
      .sessions
      .lock()
      .expect("session lock")
      .insert(id.clone(), session.clone());
    self
      .messages
      .lock()
      .expect("message lock")
      .entry(id)
      .or_default();
    self
      .providers
      .lock()
      .expect("provider lock")
      .insert(session.id.clone(), provider);

    session
  }

  pub fn update_session(
    &self,
    id: &str,
    status: SessionStatus,
    title: Option<String>,
    cwd: Option<String>,
  ) -> Option<SessionInfo> {
    let mut sessions = self.sessions.lock().expect("session lock");
    let session = sessions.get_mut(id)?;
    session.status = status;
    if let Some(title) = title {
      session.title = title;
    }
    if let Some(cwd) = cwd {
      session.cwd = Some(cwd);
    }
    session.updated_at = now_ms();
    Some(session.clone())
  }

  pub fn add_message(&self, id: &str, message: Value) {
    let mut messages = self.messages.lock().expect("message lock");
    messages.entry(id.to_string()).or_default().push(message);
  }

  pub fn get_provider(&self, id: &str) -> Option<ProviderSettings> {
    self
      .providers
      .lock()
      .expect("provider lock")
      .get(id)
      .cloned()
  }

  pub fn list_recent_cwds(&self, limit: usize) -> Vec<String> {
    let mut list: Vec<SessionInfo> = self
      .sessions
      .lock()
      .expect("session lock")
      .values()
      .cloned()
      .collect();
    list.sort_by_key(|session| session.updated_at);
    list.reverse();

    let mut seen = HashSet::new();
    let mut result = Vec::new();
    for session in list {
      if let Some(cwd) = session.cwd {
        if seen.insert(cwd.clone()) {
          result.push(cwd);
        }
      }
      if result.len() >= limit {
        break;
      }
    }
    result
  }

  pub fn register_permission(
    &self,
    tool_use_id: String,
    sender: oneshot::Sender<Value>,
  ) -> Result<(), String> {
    let mut pending = self.pending_permissions.lock().expect("permission lock");
    if pending.contains_key(&tool_use_id) {
      return Err("permission request already pending".into());
    }
    pending.insert(tool_use_id, sender);
    Ok(())
  }

  pub fn resolve_permission(&self, tool_use_id: &str, result: Value) -> bool {
    let mut pending = self.pending_permissions.lock().expect("permission lock");
    if let Some(sender) = pending.remove(tool_use_id) {
      let _ = sender.send(result);
      return true;
    }
    false
  }

  pub fn delete_session(&self, id: &str) {
    self.sessions.lock().expect("session lock").remove(id);
    self.messages.lock().expect("message lock").remove(id);
    self.providers.lock().expect("provider lock").remove(id);
  }
}

fn now_ms() -> i64 {
  SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .map(|duration| duration.as_millis() as i64)
    .unwrap_or(0)
}
