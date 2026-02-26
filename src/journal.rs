use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct JournalEntry {
    pub timestamp: DateTime<Local>,
    pub operations: Vec<Operation>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Operation {
    pub from: PathBuf,
    pub to: PathBuf,
    pub op_type: OpType,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum OpType {
    Move,
    HardLink(PathBuf),
}

impl JournalEntry {
    pub fn new() -> Self {
        Self {
            timestamp: Local::now(),
            operations: Vec::new(),
        }
    }

    pub fn save(&self, path: PathBuf) -> anyhow::Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn load(path: PathBuf) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let entry: JournalEntry = serde_json::from_str(&content)?;
        Ok(entry)
    }
}
