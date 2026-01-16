use std::process::Command;

use super::ToolExecutionResult;

pub fn run_command(command: &str, cwd: Option<&str>) -> Result<ToolExecutionResult, String> {
  let mut cmd = Command::new("sh");
  cmd.arg("-lc").arg(command);
  if let Some(cwd) = cwd {
    cmd.current_dir(cwd);
  }
  let output = cmd.output().map_err(|e| format!("Command failed: {}", e))?;
  let stdout = String::from_utf8_lossy(&output.stdout).to_string();
  let stderr = String::from_utf8_lossy(&output.stderr).to_string();
  let combined = format_output(&stdout, &stderr);
  Ok(ToolExecutionResult {
    content: combined,
    is_error: !output.status.success(),
  })
}

pub fn grep(pattern: &str, path: Option<&str>) -> Result<ToolExecutionResult, String> {
  let mut cmd = Command::new("rg");
  cmd.arg("--line-number").arg("--no-heading").arg(pattern);
  if let Some(path) = path {
    cmd.arg(path);
  }
  let output = cmd.output().map_err(|e| format!("rg failed: {}", e))?;
  if !output.status.success() {
    if output.status.code() == Some(1) {
      return Ok(ToolExecutionResult { content: String::new(), is_error: false });
    }
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    return Ok(ToolExecutionResult { content: stderr, is_error: true });
  }
  let stdout = String::from_utf8_lossy(&output.stdout).to_string();
  Ok(ToolExecutionResult { content: stdout, is_error: false })
}

fn format_output(stdout: &str, stderr: &str) -> String {
  match (stdout.trim().is_empty(), stderr.trim().is_empty()) {
    (true, true) => String::new(),
    (false, true) => stdout.to_string(),
    (true, false) => stderr.to_string(),
    (false, false) => format!("{}\n{}", stdout, stderr),
  }
}
