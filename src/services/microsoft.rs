use base64::Engine;
use base64::engine::general_purpose;
use reqwest::blocking::Client;
use serde_json::json;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::services::Translator;

const EDGE_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/137.0.0.0 Safari/537.36 Edg/137.0.0.0";

pub struct MicrosoftTranslator {
    token: String,
}

impl MicrosoftTranslator {
    pub fn new() -> MicrosoftTranslator {
        Self {
            token: String::new(),
        }
    }

    fn try_parse_jwt(token: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return Err("Invalid part length".into());
        }

        let base64_engine = general_purpose::URL_SAFE_NO_PAD;
        let decoded = base64_engine.decode(parts[1])?;
        let json_payload = String::from_utf8(decoded)?;
        serde_json::from_str(&json_payload).map_err(Into::into)
    }

    fn cached_token(&self) -> Option<serde_json::Value> {
        if let Ok(jwt) = Self::try_parse_jwt(&self.token) {
            if let Some(exp) = jwt.get("exp").map(|v| v.as_u64()).flatten() {
                let current_timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or(Duration::from_secs(0))
                    .as_secs();
                if current_timestamp + 10 < exp {
                    return Some(jwt);
                }
            }
        }
        None
    }

    fn refresh_token(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(_) = self.cached_token() {
            return Ok(());
        }

        let token = Client::new()
            .get("https://edge.microsoft.com/translate/auth")
            .header("User-Agent", EDGE_USER_AGENT)
            .send()?
            .text()?;

        match Self::try_parse_jwt(&token) {
            Ok(_) => {
                self.token = token;
                Ok(())
            }
            Err(e) => Err(e),
        }
    }
}

impl Translator for MicrosoftTranslator {
    fn auth_required(&self) -> bool {
        self.cached_token().is_none()
    }
    fn auth(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.refresh_token()
    }
    fn translate(&self, query: &str, to_lang: &str) -> Result<String, Box<dyn std::error::Error>> {
        let response = Client::new()
        .post(format!("https://api-edge.cognitive.microsofttranslator.com/translate?from=&to={}&api-version=3.0&includeSentenceLength=true&textType=html", to_lang))
        .header("Content-Type", "application/json")
        .header("Ocp-Apim-Subscription-Key", &self.token)
        .header("User-Agent", EDGE_USER_AGENT)
        .bearer_auth(&self.token)
        .body(serde_json::to_string(&json!([{"Text": query}]))?)
        .send()?;
        let text = response.text()?;
        let value: serde_json::Value = serde_json::from_str(&text)?;

        value
            .get(0)
            .map(|v| v.get("translations"))
            .flatten()
            .map(|v| v.get(0))
            .flatten()
            .map(|v| v.get("text"))
            .flatten()
            .map(|v| v.as_str())
            .flatten()
            .map(|v| v.to_string())
            .ok_or("Translation result parse failed".into())
    }
}
