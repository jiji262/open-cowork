use std::fs;
use std::path::Path;

use glob::glob;

pub fn read_file(path: &Path) -> Result<String, String> {
  fs::read_to_string(path).map_err(|e| format!("Read failed: {}", e))
}

pub fn write_file(path: &Path, content: &str) -> Result<(), String> {
  fs::write(path, content).map_err(|e| format!("Write failed: {}", e))
}

pub fn edit_file(path: &Path, old_string: &str, new_string: &str) -> Result<(), String> {
  let content = fs::read_to_string(path).map_err(|e| format!("Read failed: {}", e))?;
  if !content.contains(old_string) {
    return Err("Old string not found in file.".into());
  }
  let updated = content.replacen(old_string, new_string, 1);
  fs::write(path, updated).map_err(|e| format!("Write failed: {}", e))
}

pub fn glob_paths(pattern: &str, base: Option<&str>) -> Result<Vec<String>, String> {
  let pattern = if let Some(base) = base {
    let base_path = Path::new(base);
    if base_path.is_absolute() {
      base_path.join(pattern).to_string_lossy().to_string()
    } else {
      Path::new(".").join(base_path).join(pattern).to_string_lossy().to_string()
    }
  } else {
    pattern.to_string()
  };

  let mut results = Vec::new();
  for entry in glob(&pattern).map_err(|e| format!("Glob failed: {}", e))? {
    match entry {
      Ok(path) => results.push(path.to_string_lossy().to_string()),
      Err(error) => return Err(format!("Glob failed: {}", error)),
    }
  }
  Ok(results)
}

#[cfg(test)]
mod tests {
  use super::*;
  use tempfile::tempdir;

  #[test]
  fn read_file_works() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("a.txt");
    fs::write(&file, "hello").unwrap();
    let got = read_file(&file).unwrap();
    assert_eq!(got, "hello");
  }
}
