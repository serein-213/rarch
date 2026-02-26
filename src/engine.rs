use crate::ai::AiOracle;
use crate::config::{Config, Rule};
use crate::journal::{JournalEntry, OpType, Operation};
use chrono::{Duration, Utc};
use fs_extra::file::move_file;
use fs_extra::file::CopyOptions;
use rayon::prelude::*;
use regex::Regex;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use walkdir::WalkDir;

pub struct Engine {
    config: Arc<Config>,
    base_dir: PathBuf,
    ai: Arc<Option<AiOracle>>,
}

impl Engine {
    pub fn new(config: Config, base_dir: PathBuf) -> Self {
        let ai = Arc::new(if config.ai_api_base.is_empty() {
            None
        } else {
            Some(AiOracle::new(config.ai_api_base.clone(), config.ai_model.clone()))
        });
        Self {
            config: Arc::new(config),
            base_dir,
            ai,
        }
    }

    pub fn process_single_file<F>(&self, path: PathBuf, reporter: Option<F>) -> anyhow::Result<Option<Operation>> 
    where F: Fn(&str) + Clone
    {
        if !path.is_file() {
            return Ok(None);
        }

        if let Some(rule) = self.match_rule(&path, reporter.clone()) {
            let target_path = self.resolve_target_path(rule, &path, reporter);

            // Avoid moving if it's already in the right place
            if path == target_path {
                return Ok(None);
            }

            return Ok(Some(Operation {
                from: path,
                to: target_path,
                op_type: OpType::Move, // In watch mode, we simplify to Move for now
                rule_name: Some(rule.name.clone()),
            }));
        }
        Ok(None)
    }

    fn resolve_target_path<F>(&self, rule: &Rule, path: &Path, reporter: Option<F>) -> PathBuf 
    where F: Fn(&str) + Clone
    {
        let has_filename_placeholder = rule.target.contains("${ai_name}") 
                                      || rule.target.contains("${ext}")
                                      || rule.target.contains("${name}")
                                      || rule.target.contains("${filename}");

        let resolved_target = self.resolve_placeholders(rule, path, reporter);
        
        if has_filename_placeholder {
            if Path::new(&resolved_target).is_absolute() {
                PathBuf::from(resolved_target)
            } else {
                self.base_dir.join(resolved_target)
            }
        } else {
            let target_dir = if Path::new(&resolved_target).is_absolute() {
                PathBuf::from(resolved_target)
            } else {
                self.base_dir.join(resolved_target)
            };
            target_dir.join(path.file_name().unwrap())
        }
    }

    pub fn resolve_placeholders<F>(&self, rule: &Rule, path: &Path, reporter: Option<F>) -> String 
    where F: Fn(&str) + Clone
    {
        let mut resolved = rule.target.clone();
        
        let stem = path.file_stem().unwrap_or_default().to_string_lossy();
        let filename = path.file_name().unwrap_or_default().to_string_lossy();

        // Basic filename placeholders
        resolved = resolved.replace("${name}", &stem);
        resolved = resolved.replace("${filename}", &filename);

        // Check if we need AI renaming
        if resolved.contains("${ai_name}") {
            if let (Some(filename_str), Some(ai_oracle)) = (path.file_name().and_then(|s| s.to_str()), self.ai.as_ref()) {
                let context = rule.ai_rename_prompt.as_deref()
                    .or(rule.ai_prompt.as_deref())
                    .unwrap_or("Suggest a descriptive filename without extension");
                
                // Read snippet for better context
                let content_snippet = if let Ok(mut file) = std::fs::File::open(path) {
                    let mut buffer = [0; 512];
                    if let Ok(n) = file.read(&mut buffer) {
                        Some(String::from_utf8_lossy(&buffer[..n]).to_string())
                    } else {
                        None
                    }
                } else {
                    None
                };

                let suggested = ai_oracle.suggest_name(filename_str, content_snippet.as_deref(), context, reporter);
                resolved = resolved.replace("${ai_name}", &suggested);
            } else {
                // Fallback to original stem if AI is disabled or unavailable
                resolved = resolved.replace("${ai_name}", &stem);
            }
        }

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

    pub fn dry_run<F>(&self, on_progress: F) -> anyhow::Result<Vec<Operation>>
    where
        F: Fn(usize, usize, String) + Send + Sync + Clone,
    {
        let files: Vec<PathBuf> = WalkDir::new(&self.base_dir)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .map(|e| e.path().to_path_buf())
            .collect();

        let total = files.len();
        let current = Arc::new(Mutex::new(0));

        let seen_hashes: Arc<Mutex<HashMap<String, PathBuf>>> =
            Arc::new(Mutex::new(HashMap::new()));
        
        // We need to clone the closure if we want to use it in into_par_iter
        // but closures usually aren't Clone. Instead, use a shared wrapper.

        let ops: Vec<Operation> = files
            .into_par_iter()
            .enumerate()
            .filter_map(|(_idx, path)| {
                let progress_cb = on_progress.clone();
                let reporter = {
                    let progress_cb = progress_cb.clone();
                    let current = current.clone();
                    move |msg: &str| {
                        let count = current.lock().unwrap();
                        progress_cb(*count, total, msg.to_string());
                    }
                };
                
                let res = if let Some(rule) = self.match_rule(&path, Some(reporter.clone())) {
                    let target_path = self.resolve_target_path(rule, &path, Some(reporter));

                    // Deduplication Logic
                    let op_type = if let Ok(hash) = Self::calculate_hash(&path) {
                        let mut hashes = seen_hashes.lock().unwrap();
                        if let Some(original_target) = hashes.get(&hash) {
                            OpType::HardLink(original_target.clone())
                        } else {
                            hashes.insert(hash, target_path.clone());
                            OpType::Move
                        }
                    } else {
                        OpType::Move
                    };

                    Some(Operation {
                        from: path.clone(),
                        to: target_path,
                        op_type,
                        rule_name: Some(rule.name.clone()),
                    })
                } else {
                    None
                };

                let mut count = current.lock().unwrap();
                *count += 1;
                progress_cb(*count, total, format!("Analyzed: {:?}", path.file_name().unwrap_or_default()));
                
                res
            })
            .collect();

        Ok(ops)
    }

    pub fn execute<F>(&self, journal_path: Option<PathBuf>, mut on_progress: F) -> anyhow::Result<JournalEntry>
    where
        F: FnMut(usize, usize, String),
    {
        let ops = self.dry_run(|_, _, _| {})?;
        let total = ops.len();
        let mut journal = JournalEntry::new();
        let options = CopyOptions::new();

        for (i, op) in ops.into_iter().enumerate() {
            let target_parent = op.to.parent().expect("Target path has no parent");
            if !target_parent.exists() {
                if let Err(e) = std::fs::create_dir_all(target_parent) {
                   on_progress(i + 1, total, format!("Error (Dir): {:?}", e));
                   continue;
                }
            }

            let final_to = match self.handle_conflict(&op) {
                Ok(Some(path)) => path,
                Ok(None) => {
                    on_progress(i + 1, total, format!("Skipped (Conflict): {:?}", op.from.file_name().unwrap()));
                    continue;
                }
                Err(e) => {
                    on_progress(i + 1, total, format!("Error (Conflict): {:?}", e));
                    continue;
                }
            };

            let op_result = match &op.op_type {
                OpType::Move => move_file(&op.from, &final_to, &options).map(|_| ()),
                OpType::HardLink(original_path) => {
                    if op.from.exists() {
                        let res = std::fs::remove_file(&op.from)
                            .and_then(|_| std::fs::hard_link(original_path, &final_to));
                        res.map_err(|e| fs_extra::error::Error::from(e))
                    } else {
                        Ok(())
                    }
                }
            };

            match op_result {
                Ok(_) => {
                    let mut final_op = op;
                    final_op.to = final_to;
                    on_progress(i + 1, total, format!("Done: {:?}", final_op.from.file_name().unwrap()));
                    
                    // Atomic-like append to file
                    if let Some(path) = &journal_path {
                        let _ = JournalEntry::append_to_file(path, &final_op);
                    }
                    journal.operations.push(final_op);
                }
                Err(e) => {
                    on_progress(i + 1, total, format!("Error (Move): {:?} -> {:?} : {}", op.from, final_to, e));
                }
            }
        }

        Ok(journal)
    }

    pub(crate) fn handle_conflict(&self, op: &Operation) -> anyhow::Result<Option<PathBuf>> {
        if !op.to.exists() {
            return Ok(Some(op.to.clone()));
        }

        // We need the rule to check for conflict strategy
        let rule = self.match_rule::<fn(&str)>(&op.from, None).unwrap();
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

    pub fn match_rule<F>(&self, path: &Path, reporter: Option<F>) -> Option<&Rule> 
    where F: Fn(&str) + Clone
    {
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

            // Check Regex-based matching (Advanced)
            if !matched {
                if let (Some(rule_regex), Some(filename)) = (&rule.regex, path.file_name().and_then(|s| s.to_str())) {
                    if let Ok(re) = Regex::new(rule_regex) {
                        if re.is_match(filename) {
                            matched = true;
                        }
                    }
                }
            }

            // Check AI-based matching (Experimental/Smart)
            if !matched {
                if let (Some(ai_prompt), Some(filename), Some(ai_oracle)) = (&rule.ai_prompt, path.file_name().and_then(|s| s.to_str()), self.ai.as_ref()) {
                    // Try to read a snippet of content if it's likely text
                    let content_snippet = if let Ok(mut file) = std::fs::File::open(path) {
                        let mut buffer = [0; 512];
                        if let Ok(n) = file.read(&mut buffer) {
                            if std::str::from_utf8(&buffer[..n]).is_ok() {
                                Some(String::from_utf8_lossy(&buffer[..n]).to_string())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    if ai_oracle.matches_prompt(filename, content_snippet.as_deref(), ai_prompt, reporter.clone()) {
                        matched = true;
                    }
                }
            }

            if matched {
                // Apply AND filters (Size, Age)
                if let Some(min_size) = rule.min_size {
                    if size < min_size {
                        continue;
                    }
                }

                if let Some(max_age_str) = &rule.max_age {
                    if let Some(duration) = self.parse_age(max_age_str) {
                        if let Ok(modified) = metadata.modified() {
                            let duration_since_mod = Utc::now().signed_duration_since(chrono::DateTime::<Utc>::from(modified));
                            if duration_since_mod < duration {
                                continue;
                            }
                        }
                    }
                }
                
                return Some(rule);
            }
        }
        None
    }

    fn parse_age(&self, s: &str) -> Option<Duration> {
        let (num_part, unit_part) = s.split_at(s.len() - 1);
        let num = num_part.parse::<i64>().ok()?;
        match unit_part.to_lowercase().as_str() {
            "d" => Some(Duration::days(num)),
            "w" => Some(Duration::weeks(num)),
            "m" => Some(Duration::days(num * 30)), // Rough month
            "y" => Some(Duration::days(num * 365)), // Rough year
            "h" => Some(Duration::hours(num)),
            _ => None,
        }
    }
}
