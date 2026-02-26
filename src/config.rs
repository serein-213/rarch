use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub rules: Vec<Rule>,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(rename_all = "lowercase")]
pub enum ConflictStrategy {
    #[default]
    Rename,
    Overwrite,
    Skip,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Rule {
    #[allow(dead_code)]
    pub name: String,
    pub extensions: Option<Vec<String>>,
    pub target: String,
    pub min_size: Option<u64>,
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
