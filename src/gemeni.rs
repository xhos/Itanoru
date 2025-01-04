use crate::stickers::FuncResult;
use base64::{engine::general_purpose::STANDARD, Engine};
use emojis::get;
use lazy_static::lazy_static;
use log::{trace, warn};
use reqwest::Client;
use serde_json::json;
use std::{
  env, fs,
  time::{Duration, Instant},
};

lazy_static! {
  static ref API_KEY: String =
    env::var("GEMINI_TOKEN").expect("GEMINI_TOKEN environment variable not set");
}

static mut LAST_REQUEST: Option<Instant> = None;
const FALLBACK_EMOJI: &str = "⚙️";

async fn check_rate_limit() -> FuncResult<()> {
  let now = Instant::now();
  unsafe {
    if let Some(last) = LAST_REQUEST {
      if now.duration_since(last) < Duration::from_secs(4) {
        tokio::time::sleep(Duration::from_secs(4)).await;
      }
    }
    LAST_REQUEST = Some(Instant::now());
  }
  Ok(())
}

pub async fn generate_emojis_from_image(image_path: &str) -> FuncResult<Vec<String>> {
  check_rate_limit().await?;

  let client = Client::new();
  let image_data = fs::read(image_path)?;
  let base64_image = STANDARD.encode(image_data);

  let request_body = json!({
    "contents": [{
      "parts": [
        {
          "text": "You will receive a PNG image. Output 1-20 fitting emojis only. Do not output any other symbols, delimiters, or text.",
        },
        {
          "inline_data": {
            "mime_type": "image/png",
            "data": base64_image
          }
        }
      ]
    }]
  });

  let response = client
        .post(&format!(
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-flash:generateContent?key={}",
            *API_KEY
        ))
        .json(&request_body)
        .send()
        .await?;

  let response_text = response.text().await?.to_string();
  trace!("API response text: {}", response_text);
  let response_json: serde_json::Value = serde_json::from_str(&response_text)?;

  let raw_text = response_json["candidates"][0]["content"]["parts"][0]["text"]
    .as_str()
    .unwrap_or("");

  let mut emojis = Vec::new();
  let mut char_indices = raw_text.char_indices().peekable();
  while let Some((_, c)) = char_indices.next() {
    if let Some(emoji) = get(&c.to_string()) {
      if let Some((_, next_char)) = char_indices.peek() {
        if *next_char == '\u{fe0f}' || *next_char == '\u{fe0e}' {
          char_indices.next();
        }
      }
      emojis.push(emoji.as_str().to_string());
      if emojis.len() >= 20 {
        break;
      }
    }
  }

  if emojis.is_empty() {
    warn!("No emojis generated, using fallback emoji");
    emojis.push(FALLBACK_EMOJI.to_string());
  }

  Ok(emojis)
}
