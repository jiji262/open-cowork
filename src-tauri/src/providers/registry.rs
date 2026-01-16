use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
  Anthropic,
  OpenAI,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProviderConfig {
  pub id: String,
  pub kind: ProviderKind,
  pub base_url: Option<String>,
  pub default_model: String,
}

pub trait SecretStore: Send + Sync {
  fn set_key(&self, id: &str, value: &str) -> Result<(), String>;
  fn get_key(&self, id: &str) -> Result<Option<String>, String>;
}

pub struct InMemorySecretStore {
  data: std::sync::Mutex<std::collections::HashMap<String, String>>,
}

impl InMemorySecretStore {
  pub fn new() -> Self {
    Self { data: std::sync::Mutex::new(std::collections::HashMap::new()) }
  }
}

impl SecretStore for InMemorySecretStore {
  fn set_key(&self, id: &str, value: &str) -> Result<(), String> {
    self.data.lock().unwrap().insert(id.into(), value.into());
    Ok(())
  }

  fn get_key(&self, id: &str) -> Result<Option<String>, String> {
    Ok(self.data.lock().unwrap().get(id).cloned())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn secret_store_roundtrip() {
    let store = InMemorySecretStore::new();
    store.set_key("p1", "sk-test").unwrap();
    let got = store.get_key("p1").unwrap();
    assert_eq!(got, Some("sk-test".into()));
  }
}
