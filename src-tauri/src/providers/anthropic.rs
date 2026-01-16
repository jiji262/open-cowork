use serde_json::Value;

use super::adapter::{ChatRequest, ProviderAdapter};

pub struct AnthropicAdapter;

impl ProviderAdapter for AnthropicAdapter {
  fn build_request(&self, req: &ChatRequest) -> Value {
    serde_json::json!({
      "model": req.model,
      "messages": [{ "role": "user", "content": req.prompt }],
      "max_tokens": 1024
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn build_request_contains_model() {
    let adapter = AnthropicAdapter;
    let value = adapter.build_request(&ChatRequest {
      model: "claude-test".into(),
      prompt: "hi".into(),
    });
    assert_eq!(value["model"], "claude-test");
  }
}
