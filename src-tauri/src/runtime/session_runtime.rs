use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum SessionStatus {
  Idle,
  Running,
  Completed,
  Error,
}

#[derive(Debug, Clone)]
pub struct Session {
  pub id: String,
  pub status: SessionStatus,
}

pub struct Runtime {
  sessions: HashMap<String, Session>,
}

impl Runtime {
  pub fn new() -> Self {
    Self { sessions: HashMap::new() }
  }

  pub fn start_session(&mut self, id: &str) {
    self.sessions.insert(id.into(), Session { id: id.into(), status: SessionStatus::Running });
  }

  pub fn stop_session(&mut self, id: &str) {
    if let Some(s) = self.sessions.get_mut(id) {
      s.status = SessionStatus::Completed;
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn start_and_stop_session() {
    let mut rt = Runtime::new();
    rt.start_session("s1");
    assert_eq!(rt.sessions.get("s1").unwrap().status, SessionStatus::Running);
    rt.stop_session("s1");
    assert_eq!(rt.sessions.get("s1").unwrap().status, SessionStatus::Completed);
  }
}
