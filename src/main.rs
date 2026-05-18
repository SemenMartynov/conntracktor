mod accelerator;
mod app;
mod conntrack;
mod topology;
mod ui;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{io, time::Duration};

use app::App;

fn main() -> Result<()> {
    setup_terminal()?;

    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let mut app = App::new();

    // Catch the result of the event loop to ensure the terminal is always restored,
    // even if an error occurs during execution.
    let res = run_app(&mut terminal, &mut app);

    restore_terminal()?;

    if let Err(err) = res {
        eprintln!("Application error: {:?}", err);
    }

    Ok(())
}

/// Configures the terminal for the TUI by enabling raw mode and switching to the alternate screen.
fn setup_terminal() -> Result<()> {
    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen)?;
    Ok(())
}

/// Restores the terminal to its original state.
fn restore_terminal() -> Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    Ok(())
}

/// Main event loop.
fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    // Initial tick to populate baseline CPU statistics.
    app.on_tick();

    let tick_rate = Duration::from_secs(1);
    let mut last_tick = std::time::Instant::now();

    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Down | KeyCode::Char('j') => app.next(),
                    KeyCode::Up | KeyCode::Char('k') => app.previous(),
                    KeyCode::Esc | KeyCode::Char('q') => app.quit(),
                    _ => {}
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.on_tick();
            last_tick = std::time::Instant::now();
        }

        if app.should_quit {
            return Ok(());
        }
    }
}
