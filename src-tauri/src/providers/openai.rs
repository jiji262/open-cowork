use serde_json::Value;

use super::adapter::{ChatRequest, ProviderAdapter};

pub struct OpenAIAdapter;

impl ProviderAdapter for OpenAIAdapter {
  fn build_request(&self, req: &ChatRequest) -> Value {
    serde_json::json!({
      "model": req.model,
      "messages": [{ "role": "user", "content": req.prompt }]
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn build_request_contains_input() {
    let adapter = OpenAIAdapter;
    let value = adapter.build_request(&ChatRequest {
      model: "gpt-test".into(),
      prompt: "hi".into(),
    });
    assert_eq!(value["messages"][0]["role"], "user");
  }
}
