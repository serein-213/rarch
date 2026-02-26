#[cfg(feature = "ui")]
use crate::config::Config;
#[cfg(feature = "ui")]
use crate::engine::Engine;
#[cfg(feature = "ui")]
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
#[cfg(feature = "ui")]
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
    Terminal,
};
#[cfg(feature = "ui")]
use std::io;
use std::path::PathBuf;

#[cfg(feature = "ui")]
pub fn run_ui(path: PathBuf) -> anyhow::Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut logs: Vec<String> = vec!["Ready to organize.".to_string()];
    let mut progress: u16 = 0;

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([
                    Constraint::Length(3), // Header
                    Constraint::Length(3), // Progress Gauge
                    Constraint::Min(0),    // Main content (Logs)
                    Constraint::Length(3), // Footer (Hotkeys)
                ])
                .split(f.size());

            // Header
            let header = Paragraph::new("rarch - The Robust File Organizer")
                .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(header, chunks[0]);

            // Progress Gauge
            let gauge = Gauge::default()
                .block(
                    Block::default()
                        .title("Operation Progress")
                        .borders(Borders::ALL),
                )
                .gauge_style(Style::default().fg(Color::Green))
                .percent(progress);
            f.render_widget(gauge, chunks[1]);

            // Logs
            let items: Vec<ListItem> = logs
                .iter()
                .rev()
                .take(chunks[2].height as usize - 2)
                .map(|log| ListItem::new(log.as_str()))
                .collect();
            let log_list = List::new(items)
                .block(Block::default().title("System Logs").borders(Borders::ALL));
            f.render_widget(log_list, chunks[2]);

            // Footer
            let footer = Paragraph::new(" [R] Run Optimization   [U] Undo   [Q] Quit ")
                .style(Style::default().fg(Color::Yellow))
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(footer, chunks[3]);
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('r') => {
                        logs.push("Scanning directory...".to_string());
                        
                        // Use config for UI
                        if let Ok(config) = Config::from_file(PathBuf::from("rarch.toml")) {
                            let engine = Engine::new(config, path.clone());
                            
                            logs.push("Executing reorganization...".to_string());
                            let run_result = engine.execute(Some(PathBuf::from("rarch_journal.json")), |pos, total, msg| {
                                progress = ((pos as f32 / total as f32) * 100.0) as u16;
                                // We can't easily push to logs here because terminal.draw is blocking
                                // but for a simple UI it's fine for now if we don't redraw mid-loop
                                // or we could force a redraw if needed.
                            });

                            match run_result {
                                Ok(journal) => {
                                    progress = 100;
                                    logs.push(format!("Successfully moved {} files.", journal.operations.len()));
                                    for op in journal.operations.iter().take(5) {
                                        logs.push(format!("Moved: {:?}", op.from.file_name().unwrap()));
                                    }
                                    let _ = journal.save(PathBuf::from("rarch_journal.json"));
                                }
                                Err(e) => {
                                    logs.push(format!("Error: {}", e));
                                }
                            }
                        } else {
                            logs.push("Error: Could not load rarch.toml".to_string());
                        }
                    }
                    KeyCode::Char('u') => {
                        logs.push("Undoing last operation...".to_string());
                        progress = 0;
                        // Implementation here would call undo logic from main
                    }
                    _ => {}
                }
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen,)?;
    terminal.show_cursor()?;

    Ok(())
}

#[cfg(not(feature = "ui"))]
pub fn run_ui(_path: PathBuf) -> anyhow::Result<()> {
    println!("UI feature is not enabled. Recompile with --features ui");
    Ok(())
}
