use std::time::Duration;

pub async fn fetch_url(url: &str) -> Result<String, String> {
  let client = reqwest::Client::builder()
    .timeout(Duration::from_secs(30))
    .build()
    .map_err(|e| e.to_string())?;

  let response = client
    .get(url)
    .send()
    .await
    .map_err(|e| e.to_string())?;

  let status = response.status();
  let body = response.text().await.map_err(|e| e.to_string())?;
  let trimmed = if body.len() > 8000 {
    format!("{}...\n[truncated {} bytes]", &body[..8000], body.len().saturating_sub(8000))
  } else {
    body
  };

  Ok(format!("Status: {}\n\n{}", status, trimmed))
}
