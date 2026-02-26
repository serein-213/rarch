use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Default)]
pub struct Config {
    pub rules: Vec<Rule>,
    #[serde(default = "default_api_base")]
    pub ai_api_base: String,
    #[serde(default = "default_model")]
    pub ai_model: String,
}

fn default_api_base() -> String {
    "http://localhost:11434/v1".to_string()
}

fn default_model() -> String {
    "qwen2:0.5b".to_string()
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(rename_all = "lowercase")]
pub enum ConflictStrategy {
    #[default]
    Rename,
    Overwrite,
    Skip,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Rule {
    #[allow(dead_code)]
    pub name: String,
    pub extensions: Option<Vec<String>>,
    pub regex: Option<String>,
    pub ai_prompt: Option<String>,
    pub ai_rename_prompt: Option<String>,
    pub ai_extract: Option<std::collections::HashMap<String, String>>,
    pub target: String,
    pub min_size: Option<u64>,
    pub max_age: Option<String>,
    pub mime: Option<String>,
    pub r#type: Option<String>,
    pub conflict: Option<ConflictStrategy>,
}

impl Config {
    pub fn from_file(path: PathBuf) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
}
