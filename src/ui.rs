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
    widgets::{Block, Borders, Paragraph},
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

    let mut message = format!("Welcome to rarch UI! Path: {:?}", path);

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(0),
                    Constraint::Length(3),
                ])
                .split(f.size());

            let header = Paragraph::new("rarch - The Robust File Organizer")
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(header, chunks[0]);

            let main_body = Paragraph::new(format!(
                "Current Status:\n{}\n\nPress 'r' to Run, 'u' to Undo, 'q' to Quit",
                message
            ))
            .block(Block::default().title("Dashboard").borders(Borders::ALL));
            f.render_widget(main_body, chunks[1]);

            let footer = Paragraph::new("Built with Rust & Ratatui")
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(footer, chunks[2]);
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('r') => {
                        message = "Running optimization... (Done in CLI mode for now)".to_string();
                    }
                    KeyCode::Char('u') => {
                        message = "Undoing last operation...".to_string();
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
