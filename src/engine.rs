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
            let target_pattern = &rule.target;
            
            // 1. Resolve placeholders like ${ext}, ${year}, ${month}
            let resolved_target = self.resolve_placeholders(target_pattern, &path);
            
            // 2. Handle Absolute vs Relative paths
            let target_dir = if Path::new(&resolved_target).is_absolute() {
                PathBuf::from(resolved_target)
            } else {
                self.base_dir.join(resolved_target)
            };

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

    fn resolve_placeholders(&self, pattern: &str, path: &Path) -> String {
        let mut resolved = pattern.to_string();
        
        // Extension replacement
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            resolved = resolved.replace("${ext}", ext);
        }

        // Date replacements (modified time)
        if let Ok(metadata) = std::fs::metadata(path) {
            if let Ok(modified) = metadata.modified() {
                let dt: chrono::DateTime<chrono::Local> = modified.into();
                resolved = resolved.replace("${year}", &dt.format("%Y").to_string());
                resolved = resolved.replace("${month}", &dt.format("%m").to_string());
                resolved = resolved.replace("${day}", &dt.format("%d").to_string());
            }
        }

        resolved
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
                    let resolved_target = self.resolve_placeholders(&rule.target, &path);
                    let target_dir = if Path::new(&resolved_target).is_absolute() {
                        PathBuf::from(resolved_target)
                    } else {
                        self.base_dir.join(resolved_target)
                    };
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

    pub fn execute<F>(&self, mut on_progress: F) -> anyhow::Result<JournalEntry>
    where
        F: FnMut(usize, usize, String),
    {
        let ops = self.dry_run()?;
        let total = ops.len();
        let mut journal = JournalEntry::new();
        let options = CopyOptions::new();

        for (i, op) in ops.into_iter().enumerate() {
            let target_parent = op.to.parent().expect("Target path has no parent");
            if !target_parent.exists() {
                std::fs::create_dir_all(target_parent)?;
            }

            let final_to = self.handle_conflict(&op)?;
            if final_to.is_none() {
                on_progress(i + 1, total, format!("Skipped (Conflict): {:?}", op.from.file_name().unwrap()));
                continue;
            }
            let final_to = final_to.unwrap();

            match &op.op_type {
                OpType::Move => {
                    move_file(&op.from, &final_to, &options)?;
                }
                OpType::HardLink(original_path) => {
                    if op.from.exists() {
                        std::fs::remove_file(&op.from)?;
                        std::fs::hard_link(original_path, &final_to)?;
                    }
                }
            }

            let mut final_op = op;
            final_op.to = final_to;
            on_progress(i + 1, total, format!("Done: {:?}", final_op.from.file_name().unwrap()));
            journal.operations.push(final_op);
        }

        Ok(journal)
    }

    fn handle_conflict(&self, op: &Operation) -> anyhow::Result<Option<PathBuf>> {
        if !op.to.exists() {
            return Ok(Some(op.to.clone()));
        }

        // We need the rule to check for conflict strategy
        let rule = self.match_rule(&op.from).unwrap();
        let strategy = rule.conflict.as_ref().cloned().unwrap_or_default();

        match strategy {
            crate::config::ConflictStrategy::Skip => Ok(None),
            crate::config::ConflictStrategy::Overwrite => Ok(Some(op.to.clone())),
            crate::config::ConflictStrategy::Rename => {
                let stem = op.to.file_stem().unwrap().to_str().unwrap();
                let ext = op.to.extension().and_then(|e| e.to_str()).unwrap_or("");
                let parent = op.to.parent().unwrap();
                
                for i in 1..999 {
                    let new_name = if ext.is_empty() {
                        format!("{} ({})", stem, i)
                    } else {
                        format!("{} ({}).{}", stem, i, ext)
                    };
                    let new_path = parent.join(new_name);
                    if !new_path.exists() {
                        return Ok(Some(new_path));
                    }
                }
                anyhow::bail!("Too many file name conflicts for {:?}", op.to);
            }
        }
    }

    fn match_rule(&self, path: &Path) -> Option<&Rule> {
        let metadata = std::fs::metadata(path).ok()?;
        let size = metadata.len();

        // 1. Get detailed file info from content (Deep Recognition)
        let (detected_ext, detected_mime) = if let Ok(mut file) = std::fs::File::open(path) {
            let mut buffer = [0; 128];
            if let Ok(n) = file.read(&mut buffer) {
                let info = infer::get(&buffer[..n]);
                (
                    info.map(|kind| kind.extension().to_string()),
                    info.map(|kind| kind.mime_type().to_string()),
                )
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

        // 2. Get filename extension
        let file_ext = path
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_lowercase());

        for rule in &self.config.rules {
            let mut matched = false;

            // Check MIME-based matching (Modern/Robust)
            if let (Some(rule_mime), Some(actual_mime)) = (&rule.mime, &detected_mime) {
                if rule_mime == actual_mime || (rule_mime.ends_with("/*") && actual_mime.starts_with(&rule_mime[..rule_mime.len() - 1])) {
                    matched = true;
                }
            }

            // Check Preset-based matching (User-friendly)
            if !matched {
                if let (Some(rule_type), Some(actual_mime)) = (&rule.r#type, &detected_mime) {
                    let type_match = match rule_type.as_str() {
                        "image" => actual_mime.starts_with("image/"),
                        "video" => actual_mime.starts_with("video/"),
                        "audio" => actual_mime.starts_with("audio/"),
                        "document" => {
                            actual_mime.contains("pdf")
                                || actual_mime.contains("word")
                                || actual_mime.contains("text")
                        }
                        _ => false,
                    };
                    if type_match {
                        matched = true;
                    }
                }
            }

            // Check Extension-based matching (Classic/Fallback)
            if !matched {
                if let Some(rule_exts) = &rule.extensions {
                    matched = rule_exts.iter().any(|e| {
                        let e_low = e.to_lowercase();
                        Some(&e_low) == detected_ext.as_ref() || Some(e_low) == file_ext
                    });
                }
            }

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
