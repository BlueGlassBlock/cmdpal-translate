use crate::services::Translator;
use reqwest;
use serde_json;

pub struct GoogleTranslator;

impl Translator for GoogleTranslator {
    fn translate(&self, query: &str, to_lang: &str) -> Result<String, Box<dyn std::error::Error>> {
        let url = reqwest::Url::parse_with_params(
            "https://translate.googleapis.com/translate_a/single?client=gtx&sl=auto&dt=t&strip=1&nonced=1",
            &[("q", query), ("tl", to_lang)],
        )?;

        let client = reqwest::blocking::Client::new();
        let response: serde_json::Value = client.get(url).send()?.json()?;

        response
            .get(0)
            .and_then(|v| v.get(0))
            .and_then(|v| v.get(0))
            .and_then(|v| v.as_str())
            .map(String::from)
            .ok_or_else(|| "No translation found".into())
    }
}
