use crate::config::{Config, Rule};
use crate::journal::{JournalEntry, OpType, Operation};
use fs_extra::file::move_file;
use fs_extra::file::CopyOptions;
use rayon::prelude::*;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use walkdir::WalkDir;

pub struct Engine {
    config: Arc<Config>,
    base_dir: PathBuf,
}

impl Engine {
    pub fn new(config: Config, base_dir: PathBuf) -> Self {
        Self {
            config: Arc::new(config),
            base_dir,
        }
    }

    pub fn process_single_file(&self, path: PathBuf) -> anyhow::Result<Option<Operation>> {
        if !path.is_file() {
            return Ok(None);
        }

        if let Some(rule) = self.match_rule(&path) {
            let target_dir = self.base_dir.join(&rule.target);
            let target_path = target_dir.join(path.file_name().unwrap());

            // Avoid moving if it's already in the right place
            if path == target_path {
                return Ok(None);
            }

            return Ok(Some(Operation {
                from: path,
                to: target_path,
                op_type: OpType::Move, // In watch mode, we simplify to Move for now
            }));
        }
        Ok(None)
    }

    fn calculate_hash(path: &Path) -> anyhow::Result<String> {
        let mut file = std::fs::File::open(path)?;
        let mut hasher = Sha256::new();
        let mut buffer = [0; 4096];
        loop {
            let n = file.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
        }
        Ok(hex::encode(hasher.finalize()))
    }

    pub fn dry_run(&self) -> anyhow::Result<Vec<Operation>> {
        let files: Vec<PathBuf> = WalkDir::new(&self.base_dir)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .map(|e| e.path().to_path_buf())
            .collect();

        let seen_hashes: Arc<Mutex<HashMap<String, PathBuf>>> =
            Arc::new(Mutex::new(HashMap::new()));

        let ops: Vec<Operation> = files
            .into_par_iter()
            .filter_map(|path| {
                if let Some(rule) = self.match_rule(&path) {
                    let target_dir = self.base_dir.join(&rule.target);
                    let target_path = target_dir.join(path.file_name().unwrap());

                    // Deduplication Logic
                    if let Ok(hash) = Self::calculate_hash(&path) {
                        let mut hashes = seen_hashes.lock().unwrap();
                        if let Some(original_target) = hashes.get(&hash) {
                            return Some(Operation {
                                from: path,
                                to: target_path,
                                op_type: OpType::HardLink(original_target.clone()),
                            });
                        } else {
                            hashes.insert(hash, target_path.clone());
                        }
                    }

                    return Some(Operation {
                        from: path,
                        to: target_path,
                        op_type: OpType::Move,
                    });
                }
                None
            })
            .collect();

        Ok(ops)
    }

    pub fn execute(&self) -> anyhow::Result<JournalEntry> {
        let ops = self.dry_run()?;
        let mut journal = JournalEntry::new();
        let options = CopyOptions::new();

        for op in ops {
            let target_parent = op.to.parent().expect("Target path has no parent");
            if !target_parent.exists() {
                std::fs::create_dir_all(target_parent)?;
            }

            match &op.op_type {
                OpType::Move => {
                    move_file(&op.from, &op.to, &options)?;
                }
                OpType::HardLink(original_path) => {
                    // 1. Remove the duplicate file
                    std::fs::remove_file(&op.from)?;
                    // 2. Create a hard link from the first moved instance to the new target_path
                    std::fs::hard_link(original_path, &op.to)?;
                }
            }
            journal.operations.push(op);
        }

        Ok(journal)
    }

    fn match_rule(&self, path: &Path) -> Option<&Rule> {
        let metadata = std::fs::metadata(path).ok()?;
        let size = metadata.len();

        // 1. Try to detect extension from content (Deep Recognition)
        let detected_ext = if let Ok(mut file) = std::fs::File::open(path) {
            let mut buffer = [0; 128];
            if let Ok(n) = file.read(&mut buffer) {
                infer::get(&buffer[..n]).map(|kind| kind.extension().to_string())
            } else {
                None
            }
        } else {
            None
        };

        // 2. Fallback to filename extension
        let file_ext = path
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_lowercase());

        for rule in &self.config.rules {
            let matched = rule.extensions.iter().any(|e| {
                let e_low = e.to_lowercase();
                // Match if either content-detected extension OR filename extension matches
                Some(&e_low) == detected_ext.as_ref() || Some(e_low) == file_ext
            });

            if matched {
                if let Some(min_size) = rule.min_size {
                    if size < min_size {
                        continue;
                    }
                }
                return Some(rule);
            }
        }
        None
    }
}
