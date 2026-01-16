const MAX_TOOL_ITERATIONS_ENV: &str = "OPEN_COWORK_MAX_TOOL_ITERATIONS";

pub fn max_tool_iterations() -> usize {
  let value = std::env::var(MAX_TOOL_ITERATIONS_ENV).ok();
  parse_max_tool_iterations(value.as_deref())
}

fn parse_max_tool_iterations(value: Option<&str>) -> usize {
  value
    .and_then(|raw| {
      let trimmed = raw.trim();
      if trimmed.is_empty() {
        None
      } else {
        trimmed.parse::<usize>().ok()
      }
    })
    .unwrap_or(0)
}

pub fn should_stop_tool_loop(iterations: usize, max_iterations: usize) -> bool {
  max_iterations > 0 && iterations >= max_iterations
}

#[cfg(test)]
mod tests {
  use super::{parse_max_tool_iterations, should_stop_tool_loop};

  #[test]
  fn parse_max_tool_iterations_defaults_to_zero() {
    assert_eq!(parse_max_tool_iterations(None), 0);
    assert_eq!(parse_max_tool_iterations(Some("")), 0);
    assert_eq!(parse_max_tool_iterations(Some("not-a-number")), 0);
  }

  #[test]
  fn parse_max_tool_iterations_parses_valid_numbers() {
    assert_eq!(parse_max_tool_iterations(Some("6")), 6);
    assert_eq!(parse_max_tool_iterations(Some(" 12 ")), 12);
  }

  #[test]
  fn should_stop_tool_loop_respects_limit() {
    assert!(!should_stop_tool_loop(0, 3));
    assert!(!should_stop_tool_loop(2, 3));
    assert!(should_stop_tool_loop(3, 3));
    assert!(should_stop_tool_loop(4, 3));
  }

  #[test]
  fn should_stop_tool_loop_unlimited_when_zero() {
    assert!(!should_stop_tool_loop(0, 0));
    assert!(!should_stop_tool_loop(100, 0));
  }
}
