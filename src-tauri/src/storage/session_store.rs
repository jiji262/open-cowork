use rusqlite::{Connection, Result};

#[derive(Debug, Clone)]
pub struct SessionRow {
  pub id: String,
  pub title: String,
  pub status: String,
}

pub struct SessionStore {
  conn: Connection,
}

impl SessionStore {
  pub fn new_in_memory() -> Result<Self> {
    let conn = Connection::open_in_memory()?;
    conn.execute(
      "create table sessions(id text primary key, title text, status text)",
      [],
    )?;
    Ok(Self { conn })
  }

  pub fn insert_session(&self, id: &str, title: &str, status: &str) -> Result<()> {
    self.conn.execute(
      "insert into sessions(id,title,status) values (?1,?2,?3)",
      (id, title, status),
    )?;
    Ok(())
  }

  pub fn list_sessions(&self) -> Result<Vec<SessionRow>> {
    let mut stmt = self.conn.prepare("select id,title,status from sessions order by id")?;
    let rows = stmt
      .query_map([], |row| {
        Ok(SessionRow {
          id: row.get(0)?,
          title: row.get(1)?,
          status: row.get(2)?,
        })
      })?
      .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn insert_and_list() {
    let store = SessionStore::new_in_memory().unwrap();
    store.insert_session("s1", "Test", "idle").unwrap();
    let rows = store.list_sessions().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, "s1");
  }
}
