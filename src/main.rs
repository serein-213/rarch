mod config;
mod engine;
mod journal;
mod ui;

use clap::{Parser, Subcommand};
use config::Config;
use engine::Engine;
use fs_extra::file::move_file;
use fs_extra::file::CopyOptions;
use indicatif::{ProgressBar, ProgressStyle};
use journal::{JournalEntry, OpType};
use notify::{Config as NotifyConfig, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::mpsc::channel;

#[derive(Parser)]
#[command(name = "rarch")]
#[command(about = "A robust file organizer written in Rust", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Organize files in a directory based on rules
    Run {
        /// Path to the configuration file
        #[arg(short, long, default_value = "rarch.toml")]
        config: PathBuf,

        /// Directory to organize
        #[arg(short, long, default_value = ".")]
        path: PathBuf,

        /// Preview changes without executing
        #[arg(short, long)]
        dry_run: bool,

        /// Automatically proceed with changes without confirmation
        #[arg(short, long)]
        yes: bool,
    },
    /// Undo the last organization operation
    Undo {
        /// Path to the journal file
        #[arg(default_value = "rarch_journal.json")]
        journal: PathBuf,
    },
    /// Launch the interactive TUI
    Ui {
        /// Directory to manage
        #[arg(short, long, default_value = ".")]
        path: PathBuf,
    },
    /// Watch a directory and organize files in real-time
    Watch {
        /// Path to the configuration file
        #[arg(short, long, default_value = "rarch.toml")]
        config: PathBuf,

        /// Directory to watch
        #[arg(short, long, default_value = ".")]
        path: PathBuf,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run {
            config,
            path,
            dry_run,
            yes,
        } => {
            let config = Config::from_file(config)?;
            let engine = Engine::new(config, path);

            if dry_run {
                println!("--- Dry Run (No changes will be made) ---");
                let ops = engine.dry_run()?;
                if ops.is_empty() {
                    println!("No files matched any rules.");
                } else {
                    for op in ops {
                        println!("Match: {:?} -> {:?}", op.from, op.to);
                    }
                }
            } else {
                let ops = engine.dry_run()?;
                if ops.is_empty() {
                    println!("No files to move.");
                    return Ok(());
                }

                if !yes {
                    println!("About to move {} files.", ops.len());
                    print!("Do you want to proceed? [y/N] ");
                    use std::io::Write;
                    std::io::stdout().flush()?;
                    let mut input = String::new();
                    std::io::stdin().read_line(&mut input)?;
                    if !input.trim().eq_ignore_ascii_case("y") {
                        println!("Aborted.");
                        return Ok(());
                    }
                }

                let pb = ProgressBar::new(ops.len() as u64);
                pb.set_style(
                    ProgressStyle::with_template(
                        "[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}",
                    )
                    .unwrap()
                    .progress_chars("##-"),
                );

                let journal = engine.execute(|pos, _total, msg| {
                    pb.set_position(pos as u64);
                    pb.set_message(msg);
                })?;

                pb.finish_with_message("Done!");

                if journal.operations.is_empty() {
                    println!("No actions were performed.");
                } else {
                    println!("\nSuccessfully organized {} files.", journal.operations.len());
                    journal.save(PathBuf::from("rarch_journal.json"))?;
                    println!(
                        "Journal saved to rarch_journal.json. You can undo this with 'rarch undo'."
                    );
                }
            }
        }
        Commands::Undo { journal } => {
            let journal = JournalEntry::load(journal)?;
            let options = CopyOptions::new();
            let mut count = 0;

            // Reverse order undo
            for op in journal.operations.iter().rev() {
                if op.to.exists() {
                    match &op.op_type {
                        OpType::Move | OpType::HardLink(_) => {
                            move_file(&op.to, &op.from, &options)?;
                            count += 1;
                        }
                    }
                }
            }
            println!("Undo complete. {} files restored.", count);
        }
        Commands::Ui { path } => {
            ui::run_ui(path)?;
        }
        Commands::Watch { config, path } => {
            let config_data = Config::from_file(config)?;
            let engine = Engine::new(config_data, path.clone());
            let (tx, rx) = channel();

            let mut watcher = notify::RecommendedWatcher::new(tx, NotifyConfig::default())?;
            watcher.watch(&path, RecursiveMode::NonRecursive)?;

            println!(
                "Watching {:?} for new files... (Press Ctrl+C to stop)",
                path
            );

            for res in rx {
                match res {
                    Ok(event) => {
                        if event.kind.is_create() || event.kind.is_modify() {
                            for file_path in event.paths {
                                if let Ok(Some(op)) = engine.process_single_file(file_path.clone())
                                {
                                    let options = CopyOptions::new();
                                    let target_parent = op.to.parent().unwrap();
                                    if !target_parent.exists() {
                                        std::fs::create_dir_all(target_parent)?;
                                    }
                                    if move_file(&op.from, &op.to, &options).is_ok() {
                                        println!(
                                            "Auto-organized: {:?} -> {:?}",
                                            op.from.file_name().unwrap(),
                                            op.to
                                        );
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => println!("Watch error: {:?}", e),
                }
            }
        }
    }

    Ok(())
}
