use std::collections::HashMap;
use std::time::Duration;

use futures_util::StreamExt;
use serde_json::{json, Value};
use tokio::sync::oneshot;
use tokio::time::timeout;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::events::{ClientEvent, PermissionMode, ServerEvent, SessionStatus};
use crate::providers::registry::ProviderKind;
use crate::state::{ProviderSettings, SessionState};
use crate::tools::{self, ToolExecutionResult};
use crate::config;

const PERMISSION_TIMEOUT_SECS: u64 = 600;

#[tauri::command]
pub async fn client_event(
  app: AppHandle,
  state: State<'_, SessionState>,
  event: ClientEvent,
) -> Result<(), String> {
  match event {
    ClientEvent::SessionList => {
      let sessions = state.list_sessions();
      emit(&app, ServerEvent::SessionList { sessions })
    }
    ClientEvent::SessionHistory { session_id } => {
      let status = state
        .get_session(&session_id)
        .map(|session| session.status)
        .unwrap_or(SessionStatus::Idle);
      let messages = state.get_messages(&session_id);
      emit(
        &app,
        ServerEvent::SessionHistory {
          session_id,
          status,
          messages,
        },
      )
    }
    ClientEvent::SessionStart {
      title,
      prompt,
      cwd,
      provider,
      api_key,
      model,
      base_url,
      permission_mode,
      allowed_tools: _,
    } => {
      let api_key = api_key.trim().to_string();
      let model = model.trim().to_string();
      if api_key.is_empty() || model.is_empty() {
        emit(
          &app,
          ServerEvent::RunnerError {
            session_id: None,
            message: "API Key 或 Model 不能为空。".into(),
          },
        )?;
        return Ok(());
      }

      let provider_settings = ProviderSettings {
        provider,
        api_key,
        model,
        base_url: normalize_base_url(base_url),
        permission_mode: permission_mode.unwrap_or(PermissionMode::Ask),
      };

      let session = state.create_session(title, cwd.clone(), provider_settings.clone());

      emit(
        &app,
        ServerEvent::SessionStatus {
          session_id: session.id.clone(),
          status: SessionStatus::Running,
          title: Some(session.title.clone()),
          cwd: session.cwd.clone(),
          error: None,
        },
      )?;

      record_user_prompt(&state, &session.id, &prompt);
      emit(
        &app,
        ServerEvent::StreamUserPrompt {
          session_id: session.id.clone(),
          prompt: prompt.clone(),
        },
      )?;

      let app_handle = app.clone();
      let session_id = session.id.clone();
      let provider_settings_clone = provider_settings.clone();
      tauri::async_runtime::spawn(async move {
        let state = app_handle.state::<SessionState>();
        if let Err(message) = run_session(&app_handle, state.inner(), &session_id, provider_settings_clone).await {
          let _ = emit(
            &app_handle,
            ServerEvent::RunnerError {
              session_id: Some(session_id),
              message,
            },
          );
        }
      });

      Ok(())
    }
    ClientEvent::SessionContinue { session_id, prompt } => {
      if state.get_session(&session_id).is_none() {
        return emit(
          &app,
          ServerEvent::RunnerError {
            session_id: None,
            message: "Session not found.".into(),
          },
        );
      }

      let provider_settings = match state.get_provider(&session_id) {
        Some(settings) => settings,
        None => {
          return emit(
            &app,
            ServerEvent::RunnerError {
              session_id: Some(session_id),
              message: "Session provider config missing.".into(),
            },
          );
        }
      };

      let _ = state.update_session(&session_id, SessionStatus::Running, None, None);
      emit(
        &app,
        ServerEvent::SessionStatus {
          session_id: session_id.clone(),
          status: SessionStatus::Running,
          title: None,
          cwd: None,
          error: None,
        },
      )?;

      record_user_prompt(&state, &session_id, &prompt);
      emit(
        &app,
        ServerEvent::StreamUserPrompt {
          session_id: session_id.clone(),
          prompt: prompt.clone(),
        },
      )?;

      let app_handle = app.clone();
      let session_id_clone = session_id.clone();
      tauri::async_runtime::spawn(async move {
        let state = app_handle.state::<SessionState>();
        if let Err(message) = run_session(&app_handle, state.inner(), &session_id_clone, provider_settings).await {
          let _ = emit(
            &app_handle,
            ServerEvent::RunnerError {
              session_id: Some(session_id_clone),
              message,
            },
          );
        }
      });

      Ok(())
    }
    ClientEvent::SessionStop { session_id } => {
      let updated = state.update_session(&session_id, SessionStatus::Idle, None, None);
      let session = updated.ok_or_else(|| "Session not found.".to_string())?;
      emit(
        &app,
        ServerEvent::SessionStatus {
          session_id: session.id,
          status: session.status,
          title: Some(session.title),
          cwd: session.cwd,
          error: None,
        },
      )
    }
    ClientEvent::SessionDelete { session_id } => {
      state.delete_session(&session_id);
      emit(&app, ServerEvent::SessionDeleted { session_id })
    }
    ClientEvent::PermissionResponse { tool_use_id, result, .. } => {
      state.resolve_permission(&tool_use_id, result);
      Ok(())
    }
  }
}

async fn run_session(
  app: &AppHandle,
  state: &SessionState,
  session_id: &str,
  provider: ProviderSettings,
) -> Result<(), String> {
  let max_iterations = config::max_tool_iterations();
  let mut iterations = 0usize;
  loop {
    if config::should_stop_tool_loop(iterations, max_iterations) {
      break;
    }
    iterations += 1;
    let response = match stream_model(app, state, session_id, &provider).await {
      Ok(response) => response,
      Err(message) => {
        let _ = state.update_session(session_id, SessionStatus::Error, None, None);
        emit(
          app,
          ServerEvent::SessionStatus {
            session_id: session_id.to_string(),
            status: SessionStatus::Error,
            title: None,
            cwd: None,
            error: Some(message.clone()),
          },
        )?;
        return Err(message);
      }
    };

    if response.tool_calls.is_empty() {
      let _ = state.update_session(session_id, SessionStatus::Completed, None, None);
      emit(
        app,
        ServerEvent::SessionStatus {
          session_id: session_id.to_string(),
          status: SessionStatus::Completed,
          title: None,
          cwd: None,
          error: None,
        },
      )?;
      return Ok(());
    }

    handle_tool_calls(app, state, session_id, &response.tool_calls).await?;
  }

  let message = "工具调用循环次数过多，已停止。".to_string();
  let _ = state.update_session(session_id, SessionStatus::Error, None, None);
  emit(
    app,
    ServerEvent::SessionStatus {
      session_id: session_id.to_string(),
      status: SessionStatus::Error,
      title: None,
      cwd: None,
      error: Some(message.clone()),
    },
  )?;
  Err(message)
}

async fn stream_model(
  app: &AppHandle,
  state: &SessionState,
  session_id: &str,
  provider: &ProviderSettings,
) -> Result<ModelResponse, String> {
  match provider.provider {
    ProviderKind::Anthropic => stream_anthropic(app, state, session_id, provider).await,
    ProviderKind::OpenAI => stream_openai(app, state, session_id, provider).await,
  }
}

async fn stream_openai(
  app: &AppHandle,
  state: &SessionState,
  session_id: &str,
  provider: &ProviderSettings,
) -> Result<ModelResponse, String> {
  let client = reqwest::Client::builder()
    .timeout(Duration::from_secs(120))
    .build()
    .map_err(|e| e.to_string())?;

  let url = provider
    .base_url
    .clone()
    .unwrap_or_else(|| "https://api.openai.com/v1/chat/completions".into());

  let messages = build_openai_messages(state, session_id);
  let body = json!({
    "model": provider.model,
    "messages": messages,
    "stream": true,
    "tools": openai_tools(),
    "tool_choice": "auto"
  });

  let response = client
    .post(url)
    .bearer_auth(&provider.api_key)
    .json(&body)
    .send()
    .await
    .map_err(|e| e.to_string())?;

  if !response.status().is_success() {
    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    return Err(format!("OpenAI 请求失败({}): {}", status, text));
  }

  let mut stream = response.bytes_stream();
  let mut buffer = String::new();
  let mut assistant_text = String::new();
  let mut tool_calls: Vec<ToolCallBuilder> = Vec::new();
  let mut started = false;

  while let Some(chunk) = stream.next().await {
    let chunk = chunk.map_err(|e| e.to_string())?;
    buffer.push_str(&String::from_utf8_lossy(&chunk));
    for event in drain_sse_events(&mut buffer) {
      for line in event.lines() {
        let data = match line.trim().strip_prefix("data:") {
          Some(value) => value.trim(),
          None => continue,
        };
        if data == "[DONE]" {
          break;
        }
        let payload: Value = serde_json::from_str(data).map_err(|e| e.to_string())?;
        if let Some(delta) = payload.pointer("/choices/0/delta") {
          if let Some(content) = delta.get("content").and_then(Value::as_str) {
            if !started {
              started = true;
              emit_stream_event(app, session_id, "content_block_start", None)?;
            }
            assistant_text.push_str(content);
            emit_stream_event(
              app,
              session_id,
              "content_block_delta",
              Some(json!({ "type": "text_delta", "text": content })),
            )?;
          }
          if let Some(calls) = delta.get("tool_calls").and_then(Value::as_array) {
            for call in calls {
              let index = call.get("index").and_then(Value::as_u64).unwrap_or(0) as usize;
              if tool_calls.len() <= index {
                tool_calls.resize_with(index + 1, ToolCallBuilder::default);
              }
              let entry = &mut tool_calls[index];
              if let Some(id) = call.get("id").and_then(Value::as_str) {
                entry.id = Some(id.to_string());
              }
              if let Some(name) = call.pointer("/function/name").and_then(Value::as_str) {
                entry.name = Some(name.to_string());
              }
              if let Some(args) = call.pointer("/function/arguments").and_then(Value::as_str) {
                entry.arguments.push_str(args);
              }
            }
          }
        }
      }
    }
  }

  if started {
    emit_stream_event(app, session_id, "content_block_stop", None)?;
  }

  let tool_calls = finalize_tool_calls(tool_calls);
  let content_blocks = build_content_blocks(&assistant_text, &tool_calls);
  emit_assistant_message(app, state, session_id, content_blocks)?;

  Ok(ModelResponse { tool_calls })
}

async fn stream_anthropic(
  app: &AppHandle,
  state: &SessionState,
  session_id: &str,
  provider: &ProviderSettings,
) -> Result<ModelResponse, String> {
  let client = reqwest::Client::builder()
    .timeout(Duration::from_secs(120))
    .build()
    .map_err(|e| e.to_string())?;

  let url = provider
    .base_url
    .clone()
    .unwrap_or_else(|| "https://api.anthropic.com/v1/messages".into());

  let messages = build_anthropic_messages(state, session_id);
  let body = json!({
    "model": provider.model,
    "messages": messages,
    "stream": true,
    "max_tokens": 1024,
    "tools": anthropic_tools()
  });

  let response = client
    .post(url)
    .header("x-api-key", &provider.api_key)
    .header("anthropic-version", "2023-06-01")
    .json(&body)
    .send()
    .await
    .map_err(|e| e.to_string())?;

  if !response.status().is_success() {
    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    return Err(format!("Anthropic 请求失败({}): {}", status, text));
  }

  let mut stream = response.bytes_stream();
  let mut buffer = String::new();
  let mut blocks: HashMap<u64, AnthropicBlock> = HashMap::new();

  while let Some(chunk) = stream.next().await {
    let chunk = chunk.map_err(|e| e.to_string())?;
    buffer.push_str(&String::from_utf8_lossy(&chunk));
    for event in drain_sse_events(&mut buffer) {
      let mut event_type = "";
      let mut data_lines = Vec::new();
      for line in event.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("event:") {
          event_type = value.trim();
        } else if let Some(value) = line.strip_prefix("data:") {
          data_lines.push(value.trim());
        }
      }
      if data_lines.is_empty() {
        continue;
      }
      let data = data_lines.join("\n");
      let payload: Value = serde_json::from_str(&data).map_err(|e| e.to_string())?;

      match event_type {
        "content_block_start" => {
          let index = payload.get("index").and_then(Value::as_u64).unwrap_or(0);
          if let Some(block) = payload.get("content_block") {
            if block.get("type").and_then(Value::as_str) == Some("text") {
              blocks.insert(index, AnthropicBlock::Text(String::new()));
              emit_stream_event(app, session_id, "content_block_start", None)?;
            } else if block.get("type").and_then(Value::as_str) == Some("tool_use") {
              let id = block.get("id").and_then(Value::as_str).unwrap_or_default().to_string();
              let name = block.get("name").and_then(Value::as_str).unwrap_or_default().to_string();
              let input = block.get("input").cloned();
              blocks.insert(index, AnthropicBlock::ToolUse { id, name, input_json: String::new(), input });
            }
          }
        }
        "content_block_delta" => {
          let index = payload.get("index").and_then(Value::as_u64).unwrap_or(0);
          if let Some(delta) = payload.get("delta") {
            if delta.get("type").and_then(Value::as_str) == Some("text_delta") {
              let text = delta.get("text").and_then(Value::as_str).unwrap_or("");
              if let Some(AnthropicBlock::Text(current)) = blocks.get_mut(&index) {
                current.push_str(text);
              }
              emit_stream_event(
                app,
                session_id,
                "content_block_delta",
                Some(json!({ "type": "text_delta", "text": text })),
              )?;
            } else if delta.get("type").and_then(Value::as_str) == Some("input_json_delta") {
              let part = delta.get("partial_json").and_then(Value::as_str).unwrap_or("");
              if let Some(AnthropicBlock::ToolUse { input_json, .. }) = blocks.get_mut(&index) {
                input_json.push_str(part);
              }
            }
          }
        }
        "content_block_stop" => {
          emit_stream_event(app, session_id, "content_block_stop", None)?;
        }
        _ => {}
      }
    }
  }

  let mut tool_calls = Vec::new();
  let mut content_blocks = Vec::new();
  let mut ordered_keys: Vec<u64> = blocks.keys().cloned().collect();
  ordered_keys.sort_unstable();
  for key in ordered_keys {
    if let Some(block) = blocks.remove(&key) {
      match block {
        AnthropicBlock::Text(text) => {
          if !text.is_empty() {
            content_blocks.push(json!({ "type": "text", "text": text }));
          }
        }
        AnthropicBlock::ToolUse { id, name, input_json, input } => {
          let parsed = if !input_json.trim().is_empty() {
            serde_json::from_str(&input_json).unwrap_or(Value::String(input_json))
          } else {
            input.unwrap_or(Value::Null)
          };
          tool_calls.push(ToolCall { id: id.clone(), name: name.clone(), input: parsed.clone() });
          content_blocks.push(json!({ "type": "tool_use", "id": id, "name": name, "input": parsed }));
        }
      }
    }
  }

  emit_assistant_message(app, state, session_id, content_blocks)?;

  Ok(ModelResponse { tool_calls })
}

async fn handle_tool_calls(
  app: &AppHandle,
  state: &SessionState,
  session_id: &str,
  tool_calls: &[ToolCall],
) -> Result<(), String> {
  let cwd = state.get_session(session_id).and_then(|session| session.cwd);
  let permission_mode = state
    .get_provider(session_id)
    .map(|settings| settings.permission_mode)
    .unwrap_or(PermissionMode::Ask);
  for call in tool_calls {
    let permission = if permission_mode == PermissionMode::Auto && call.name != "AskUserQuestion" {
      json!({ "behavior": "allow", "updatedInput": call.input })
    } else {
      request_permission(app, state, session_id, call).await?
    };
    let behavior = permission
      .get("behavior")
      .and_then(Value::as_str)
      .unwrap_or("deny");

    if behavior != "allow" {
      let message = permission
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("User denied the request.");
      let tool_message = build_tool_result_message(&call.id, message, true);
      state.add_message(session_id, tool_message.clone());
      emit(
        app,
        ServerEvent::StreamMessage {
          session_id: session_id.to_string(),
          message: tool_message,
        },
      )?;
      continue;
    }

    let effective_input = permission
      .get("updatedInput")
      .cloned()
      .unwrap_or_else(|| call.input.clone());

    let execution = if call.name == "AskUserQuestion" {
      ToolExecutionResult {
        content: stringify_value(&effective_input),
        is_error: false,
      }
    } else {
      match tools::execute_tool(&call.name, &effective_input, cwd.as_deref()).await {
        Ok(result) => result,
        Err(error) => ToolExecutionResult { content: error, is_error: true },
      }
    };

    let tool_message = build_tool_result_message(&call.id, &execution.content, execution.is_error);
    state.add_message(session_id, tool_message.clone());
    emit(
      app,
      ServerEvent::StreamMessage {
        session_id: session_id.to_string(),
        message: tool_message,
      },
    )?;
  }
  Ok(())
}

async fn request_permission(
  app: &AppHandle,
  state: &SessionState,
  session_id: &str,
  call: &ToolCall,
) -> Result<Value, String> {
  let (sender, receiver) = oneshot::channel();
  state.register_permission(call.id.clone(), sender)?;
  emit(
    app,
    ServerEvent::PermissionRequest {
      session_id: session_id.to_string(),
      tool_use_id: call.id.clone(),
      tool_name: call.name.clone(),
      input: call.input.clone(),
    },
  )?;
  match timeout(Duration::from_secs(PERMISSION_TIMEOUT_SECS), receiver).await {
    Ok(Ok(result)) => Ok(result),
    Ok(Err(_)) => Err("Permission channel closed.".into()),
    Err(_) => Err("Permission request timed out.".into()),
  }
}

fn emit_assistant_message(
  app: &AppHandle,
  state: &SessionState,
  session_id: &str,
  content_blocks: Vec<Value>,
) -> Result<(), String> {
  if content_blocks.is_empty() {
    return Ok(());
  }
  let assistant = json!({
    "type": "assistant",
    "message": {
      "content": content_blocks
    }
  });
  state.add_message(session_id, assistant.clone());
  emit(
    app,
    ServerEvent::StreamMessage {
      session_id: session_id.to_string(),
      message: assistant,
    },
  )
}

fn emit_stream_event(
  app: &AppHandle,
  session_id: &str,
  event_type: &str,
  delta: Option<Value>,
) -> Result<(), String> {
  let mut event = json!({ "type": event_type });
  if let Some(delta) = delta {
    event["delta"] = delta;
  }
  let message = json!({ "type": "stream_event", "event": event });
  emit(
    app,
    ServerEvent::StreamMessage {
      session_id: session_id.to_string(),
      message,
    },
  )
}

fn build_tool_result_message(tool_use_id: &str, content: &str, is_error: bool) -> Value {
  let safe_content = if is_error && content.trim().is_empty() {
    "Tool execution failed."
  } else {
    content
  };
  json!({
    "type": "user",
    "message": {
      "content": [
        {
          "type": "tool_result",
          "tool_use_id": tool_use_id,
          "content": safe_content,
          "is_error": is_error
        }
      ]
    }
  })
}

fn build_content_blocks(text: &str, tool_calls: &[ToolCall]) -> Vec<Value> {
  let mut blocks = Vec::new();
  if !text.trim().is_empty() {
    blocks.push(json!({ "type": "text", "text": text }));
  }
  for call in tool_calls {
    blocks.push(json!({
      "type": "tool_use",
      "id": call.id,
      "name": call.name,
      "input": call.input
    }));
  }
  blocks
}

fn build_openai_messages(state: &SessionState, session_id: &str) -> Vec<Value> {
  let history = state.get_messages(session_id);
  let mut messages = Vec::new();

  for item in history {
    let msg_type = item.get("type").and_then(Value::as_str);
    match msg_type {
      Some("user_prompt") => {
        if let Some(prompt) = item.get("prompt").and_then(Value::as_str) {
          messages.push(json!({ "role": "user", "content": prompt }));
        }
      }
      Some("assistant") => {
        if let Some(contents) = item.pointer("/message/content").and_then(Value::as_array) {
          let mut text = String::new();
          let mut tool_calls = Vec::new();
          for content in contents {
            match content.get("type").and_then(Value::as_str) {
              Some("text") => {
                if let Some(part) = content.get("text").and_then(Value::as_str) {
                  text.push_str(part);
                }
              }
              Some("tool_use") => {
                let id = content.get("id").and_then(Value::as_str).unwrap_or_default();
                let name = content.get("name").and_then(Value::as_str).unwrap_or_default();
                let input = content.get("input").cloned().unwrap_or(Value::Null);
                tool_calls.push(json!({
                  "id": id,
                  "type": "function",
                  "function": {
                    "name": name,
                    "arguments": stringify_value(&input)
                  }
                }));
              }
              _ => {}
            }
          }
          if !tool_calls.is_empty() || !text.trim().is_empty() {
            let mut message = json!({ "role": "assistant", "content": text });
            if !tool_calls.is_empty() {
              message["tool_calls"] = Value::Array(tool_calls);
            }
            messages.push(message);
          }
        }
      }
      Some("user") => {
        if let Some(contents) = item.pointer("/message/content").and_then(Value::as_array) {
          for content in contents {
            if content.get("type").and_then(Value::as_str) == Some("tool_result") {
              let id = content.get("tool_use_id").and_then(Value::as_str).unwrap_or_default();
              let payload = content.get("content").cloned().unwrap_or(Value::String(String::new()));
              messages.push(json!({
                "role": "tool",
                "tool_call_id": id,
                "content": stringify_value(&payload)
              }));
            }
          }
        }
      }
      _ => {}
    }
  }

  messages
}

fn build_anthropic_messages(state: &SessionState, session_id: &str) -> Vec<Value> {
  let history = state.get_messages(session_id);
  let mut messages = Vec::new();
  let mut pending_tool_results: Vec<Value> = Vec::new();

  let flush_tool_results = |pending: &mut Vec<Value>, output: &mut Vec<Value>| {
    if pending.is_empty() {
      return;
    }
    output.push(json!({ "role": "user", "content": pending.clone() }));
    pending.clear();
  };

  for item in history {
    let msg_type = item.get("type").and_then(Value::as_str);
    match msg_type {
      Some("user_prompt") => {
        flush_tool_results(&mut pending_tool_results, &mut messages);
        if let Some(prompt) = item.get("prompt").and_then(Value::as_str) {
          messages.push(json!({
            "role": "user",
            "content": [{ "type": "text", "text": prompt }]
          }));
        }
      }
      Some("assistant") => {
        flush_tool_results(&mut pending_tool_results, &mut messages);
        if let Some(contents) = item.pointer("/message/content").and_then(Value::as_array) {
          messages.push(json!({ "role": "assistant", "content": contents }));
        }
      }
      Some("user") => {
        if let Some(contents) = item.pointer("/message/content").and_then(Value::as_array) {
          let only_tool_results = contents
            .iter()
            .all(|content| content.get("type").and_then(Value::as_str) == Some("tool_result"));
          if only_tool_results {
            pending_tool_results.extend(contents.iter().cloned());
          } else {
            flush_tool_results(&mut pending_tool_results, &mut messages);
            messages.push(json!({ "role": "user", "content": contents }));
          }
        }
      }
      _ => {}
    }
  }

  flush_tool_results(&mut pending_tool_results, &mut messages);

  messages
}

fn openai_tools() -> Vec<Value> {
  vec![
    tool_def("Read", "Read a file from disk.", json!({
      "type": "object",
      "properties": { "file_path": { "type": "string" } },
      "required": ["file_path"]
    })),
    tool_def("Write", "Write a file to disk.", json!({
      "type": "object",
      "properties": {
        "file_path": { "type": "string" },
        "content": { "type": "string" }
      },
      "required": ["file_path", "content"]
    })),
    tool_def("Edit", "Replace a string in a file.", json!({
      "type": "object",
      "properties": {
        "file_path": { "type": "string" },
        "old_string": { "type": "string" },
        "new_string": { "type": "string" }
      },
      "required": ["file_path", "old_string", "new_string"]
    })),
    tool_def("Bash", "Run a shell command.", json!({
      "type": "object",
      "properties": { "command": { "type": "string" } },
      "required": ["command"]
    })),
    tool_def("Glob", "Find files matching a glob pattern.", json!({
      "type": "object",
      "properties": {
        "pattern": { "type": "string" },
        "path": { "type": "string" }
      },
      "required": ["pattern"]
    })),
    tool_def("Grep", "Search for text within files.", json!({
      "type": "object",
      "properties": {
        "pattern": { "type": "string" },
        "path": { "type": "string" }
      },
      "required": ["pattern"]
    })),
    tool_def("WebFetch", "Fetch a URL over HTTP.", json!({
      "type": "object",
      "properties": { "url": { "type": "string" } },
      "required": ["url"]
    })),
    tool_def("Task", "Create a sub-task description.", json!({
      "type": "object",
      "properties": { "description": { "type": "string" } },
      "required": ["description"]
    })),
    tool_def("AskUserQuestion", "Ask user clarifying questions.", ask_user_schema()),
  ]
}

fn anthropic_tools() -> Vec<Value> {
  let tools = openai_tools();
  tools
    .into_iter()
    .filter_map(|tool| {
      let name = tool.pointer("/function/name")?.as_str()?.to_string();
      let description = tool.pointer("/function/description")?.as_str()?.to_string();
      let schema = tool.pointer("/function/parameters")?.clone();
      Some(json!({
        "name": name,
        "description": description,
        "input_schema": schema
      }))
    })
    .collect()
}

fn tool_def(name: &str, description: &str, parameters: Value) -> Value {
  json!({
    "type": "function",
    "function": {
      "name": name,
      "description": description,
      "parameters": parameters
    }
  })
}

fn ask_user_schema() -> Value {
  json!({
    "type": "object",
    "properties": {
      "questions": {
        "type": "array",
        "items": {
          "type": "object",
          "properties": {
            "question": { "type": "string" },
            "header": { "type": "string" },
            "options": {
              "type": "array",
              "items": {
                "type": "object",
                "properties": {
                  "label": { "type": "string" },
                  "description": { "type": "string" }
                },
                "required": ["label"]
              }
            },
            "multiSelect": { "type": "boolean" }
          },
          "required": ["question"]
        }
      }
    },
    "required": ["questions"]
  })
}

fn record_user_prompt(state: &SessionState, session_id: &str, prompt: &str) {
  let message = json!({ "type": "user_prompt", "prompt": prompt });
  state.add_message(session_id, message);
}

fn normalize_base_url(base_url: Option<String>) -> Option<String> {
  base_url
    .map(|value| value.trim().to_string())
    .filter(|value| !value.is_empty())
}

fn emit(app: &AppHandle, event: ServerEvent) -> Result<(), String> {
  app.emit("server-event", event).map_err(|e| e.to_string())
}

fn stringify_value(value: &Value) -> String {
  match value {
    Value::String(text) => text.clone(),
    _ => value.to_string(),
  }
}

fn drain_sse_events(buffer: &mut String) -> Vec<String> {
  let mut events = Vec::new();
  loop {
    if let Some(pos) = buffer.find("\n\n") {
      let event = buffer[..pos].to_string();
      buffer.drain(..pos + 2);
      if !event.trim().is_empty() {
        events.push(event);
      }
    } else {
      break;
    }
  }
  events
}

#[derive(Debug, Clone)]
struct ToolCall {
  id: String,
  name: String,
  input: Value,
}

struct ModelResponse {
  tool_calls: Vec<ToolCall>,
}

#[derive(Default)]
struct ToolCallBuilder {
  id: Option<String>,
  name: Option<String>,
  arguments: String,
}

fn finalize_tool_calls(builders: Vec<ToolCallBuilder>) -> Vec<ToolCall> {
  let mut calls = Vec::new();
  for (idx, builder) in builders.into_iter().enumerate() {
    let name = builder.name.unwrap_or_else(|| "UnknownTool".into());
    let id = builder.id.unwrap_or_else(|| format!("tool-{}", idx));
    let input = if builder.arguments.trim().is_empty() {
      Value::Null
    } else {
      serde_json::from_str(&builder.arguments).unwrap_or(Value::String(builder.arguments))
    };
    calls.push(ToolCall { id, name, input });
  }
  calls
}

enum AnthropicBlock {
  Text(String),
  ToolUse {
    id: String,
    name: String,
    input_json: String,
    input: Option<Value>,
  },
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::events::PermissionMode;
  use crate::providers::registry::ProviderKind;
  use crate::state::{ProviderSettings, SessionState};

  #[test]
  fn build_anthropic_messages_merges_tool_results() {
    let state = SessionState::new();
    let provider = ProviderSettings {
      provider: ProviderKind::Anthropic,
      api_key: "test".into(),
      model: "test".into(),
      base_url: None,
      permission_mode: PermissionMode::Ask,
    };
    let session = state.create_session("test".into(), None, provider);

    state.add_message(&session.id, json!({ "type": "user_prompt", "prompt": "hi" }));
    state.add_message(&session.id, json!({
      "type": "assistant",
      "message": {
        "content": [
          { "type": "tool_use", "id": "tool-1", "name": "Read", "input": { "file_path": "a.txt" } },
          { "type": "tool_use", "id": "tool-2", "name": "Read", "input": { "file_path": "b.txt" } }
        ]
      }
    }));
    state.add_message(&session.id, json!({
      "type": "user",
      "message": {
        "content": [
          { "type": "tool_result", "tool_use_id": "tool-1", "content": "ok", "is_error": false }
        ]
      }
    }));
    state.add_message(&session.id, json!({
      "type": "user",
      "message": {
        "content": [
          { "type": "tool_result", "tool_use_id": "tool-2", "content": "ok2", "is_error": false }
        ]
      }
    }));

    let messages = build_anthropic_messages(&state, &session.id);
    assert_eq!(messages.len(), 3);
    let last = messages.last().expect("expected tool result message");
    assert_eq!(last.get("role").and_then(Value::as_str), Some("user"));
    let contents = last.get("content").and_then(Value::as_array).expect("content array");
    assert_eq!(contents.len(), 2);
    assert_eq!(contents[0].get("tool_use_id").and_then(Value::as_str), Some("tool-1"));
    assert_eq!(contents[1].get("tool_use_id").and_then(Value::as_str), Some("tool-2"));
  }

  #[test]
  fn build_tool_result_message_fills_error_content() {
    let message = build_tool_result_message("tool-1", "", true);
    let content = message
      .pointer("/message/content/0/content")
      .and_then(Value::as_str)
      .unwrap_or("");
    assert!(!content.trim().is_empty());
  }
}
