use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub struct JournalEntry {
    pub timestamp: DateTime<Local>,
    pub operations: Vec<Operation>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Operation {
    pub from: PathBuf,
    pub to: PathBuf,
    pub op_type: OpType,
    pub rule_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
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

    /// Appends a single operation to a journal file in a crash-safe way (JSON Lines)
    pub fn append_to_file(path: &Path, op: &Operation) -> anyhow::Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        
        let json = serde_json::to_string(op)?;
        writeln!(file, "{}", json)?;
        Ok(())
    }

    /// Regular save for the whole structure
    pub fn save(&self, path: PathBuf) -> anyhow::Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn load(path: PathBuf) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(&path)?;
        // Try parsing as full JournalEntry (legacy) or JSON Lines
        if let Ok(entry) = serde_json::from_str::<JournalEntry>(&content) {
            return Ok(entry);
        }

        // Fallback: parse as JSON Lines
        let operations: Vec<Operation> = content
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect();
            
        Ok(Self {
            timestamp: Local::now(), // Not quite accurate but works for load
            operations,
        })
    }
}
