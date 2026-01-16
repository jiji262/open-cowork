use serde_json::Value;

pub struct ChatRequest {
  pub model: String,
  pub prompt: String,
}

pub trait ProviderAdapter {
  fn build_request(&self, req: &ChatRequest) -> Value;
}
