use serde_json::Value;
use std::path::{Path, PathBuf};

pub mod command_tools;
pub mod fs_tools;
pub mod web_tools;

pub struct ToolExecutionResult {
  pub content: String,
  pub is_error: bool,
}

pub async fn execute_tool(
  name: &str,
  input: &Value,
  cwd: Option<&str>,
) -> Result<ToolExecutionResult, String> {
  match name {
    "Read" => {
      let file_path = get_required_string(input, "file_path")?;
      let path = resolve_path(&file_path, cwd);
      let content = fs_tools::read_file(&path)?;
      Ok(ToolExecutionResult { content, is_error: false })
    }
    "Write" => {
      let file_path = get_required_string(input, "file_path")?;
      let content = get_required_string(input, "content")?;
      let path = resolve_path(&file_path, cwd);
      fs_tools::write_file(&path, &content)?;
      Ok(ToolExecutionResult {
        content: format!("Wrote {} bytes to {}", content.len(), path.display()),
        is_error: false,
      })
    }
    "Edit" => {
      let file_path = get_required_string(input, "file_path")?;
      let old_string = get_required_string(input, "old_string")?;
      let new_string = get_required_string(input, "new_string")?;
      let path = resolve_path(&file_path, cwd);
      fs_tools::edit_file(&path, &old_string, &new_string)?;
      Ok(ToolExecutionResult {
        content: format!("Updated {}", path.display()),
        is_error: false,
      })
    }
    "Bash" => {
      let command = get_required_string(input, "command")?;
      let result = command_tools::run_command(&command, cwd)?;
      Ok(result)
    }
    "Glob" => {
      let pattern = get_required_string(input, "pattern")?;
      let base = get_optional_string(input, "path")
        .or_else(|| cwd.map(|value| value.to_string()));
      let matches = fs_tools::glob_paths(&pattern, base.as_deref())?;
      Ok(ToolExecutionResult {
        content: matches.join("\n"),
        is_error: false,
      })
    }
    "Grep" => {
      let pattern = get_required_string(input, "pattern")?;
      let path = get_optional_string(input, "path")
        .or_else(|| get_optional_string(input, "file_path"))
        .or_else(|| cwd.map(|value| value.to_string()));
      let result = command_tools::grep(&pattern, path.as_deref())?;
      Ok(result)
    }
    "WebFetch" => {
      let url = get_required_string(input, "url")?;
      let content = web_tools::fetch_url(&url).await?;
      Ok(ToolExecutionResult { content, is_error: false })
    }
    "Task" => {
      let description = get_required_string(input, "description")?;
      Ok(ToolExecutionResult {
        content: format!("Task noted: {}", description),
        is_error: false,
      })
    }
    "AskUserQuestion" => Err("AskUserQuestion should be handled via permission workflow.".into()),
    other => Err(format!("Unsupported tool: {}", other)),
  }
}

fn get_required_string(input: &Value, key: &str) -> Result<String, String> {
  input
    .get(key)
    .and_then(Value::as_str)
    .map(|value| value.to_string())
    .ok_or_else(|| format!("Missing required field: {}", key))
}

fn get_optional_string(input: &Value, key: &str) -> Option<String> {
  input.get(key).and_then(Value::as_str).map(|value| value.to_string())
}

fn resolve_path(path: &str, cwd: Option<&str>) -> PathBuf {
  let candidate = Path::new(path);
  if candidate.is_absolute() {
    return candidate.to_path_buf();
  }
  match cwd {
    Some(base) => Path::new(base).join(candidate),
    None => candidate.to_path_buf(),
  }
}
