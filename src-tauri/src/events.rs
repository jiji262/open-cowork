use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::providers::registry::ProviderKind;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum PermissionMode {
  Ask,
  Auto,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
  Idle,
  Running,
  Completed,
  Error,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
  pub id: String,
  pub title: String,
  pub status: SessionStatus,
  pub cwd: Option<String>,
  pub claude_session_id: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub provider: Option<ProviderKind>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub model: Option<String>,
  pub created_at: i64,
  pub updated_at: i64,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "type", content = "payload")]
pub enum ServerEvent {
  #[serde(rename = "session.list")]
  SessionList { sessions: Vec<SessionInfo> },
  #[serde(rename = "session.history")]
  SessionHistory {
    #[serde(rename = "sessionId")]
    session_id: String,
    status: SessionStatus,
    messages: Vec<Value>,
  },
  #[serde(rename = "session.status")]
  SessionStatus {
    #[serde(rename = "sessionId")]
    session_id: String,
    status: SessionStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cwd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
  },
  #[serde(rename = "session.deleted")]
  SessionDeleted {
    #[serde(rename = "sessionId")]
    session_id: String,
  },
  #[serde(rename = "stream.message")]
  StreamMessage {
    #[serde(rename = "sessionId")]
    session_id: String,
    message: Value,
  },
  #[serde(rename = "stream.user_prompt")]
  StreamUserPrompt {
    #[serde(rename = "sessionId")]
    session_id: String,
    prompt: String,
  },
  #[serde(rename = "permission.request")]
  PermissionRequest {
    #[serde(rename = "sessionId")]
    session_id: String,
    #[serde(rename = "toolUseId")]
    tool_use_id: String,
    #[serde(rename = "toolName")]
    tool_name: String,
    input: Value,
  },
  #[serde(rename = "runner.error")]
  RunnerError {
    #[serde(rename = "sessionId")]
    session_id: Option<String>,
    message: String,
  },
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "type", content = "payload")]
pub enum ClientEvent {
  #[serde(rename = "session.list")]
  SessionList,
  #[serde(rename = "session.history")]
  SessionHistory {
    #[serde(rename = "sessionId")]
    session_id: String,
  },
  #[serde(rename = "session.start")]
  SessionStart {
    title: String,
    prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    cwd: Option<String>,
    provider: ProviderKind,
    #[serde(rename = "apiKey")]
    api_key: String,
    model: String,
    #[serde(rename = "baseUrl", skip_serializing_if = "Option::is_none")]
    base_url: Option<String>,
    #[serde(rename = "permissionMode", skip_serializing_if = "Option::is_none")]
    permission_mode: Option<PermissionMode>,
    #[serde(rename = "allowedTools", skip_serializing_if = "Option::is_none")]
    allowed_tools: Option<String>,
  },
  #[serde(rename = "session.continue")]
  SessionContinue {
    #[serde(rename = "sessionId")]
    session_id: String,
    prompt: String,
  },
  #[serde(rename = "session.stop")]
  SessionStop {
    #[serde(rename = "sessionId")]
    session_id: String,
  },
  #[serde(rename = "session.delete")]
  SessionDelete {
    #[serde(rename = "sessionId")]
    session_id: String,
  },
  #[serde(rename = "permission.response")]
  PermissionResponse {
    #[serde(rename = "sessionId")]
    session_id: String,
    #[serde(rename = "toolUseId")]
    tool_use_id: String,
    result: Value,
  },
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn server_event_serializes() {
    let event = ServerEvent::SessionList { sessions: vec![] };
    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains("\"type\":\"session.list\""));
  }
}
