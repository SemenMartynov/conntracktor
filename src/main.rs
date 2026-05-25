mod app;
mod conntrack;
mod platform;
mod topology;
mod ui;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{io, time::Duration};

use app::App;

fn main() -> Result<()> {
    setup_terminal()?;

    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    // Construct and init the application state.
    let mut app = App::new();
    app.init();

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
                    // Togle help popup
                    KeyCode::Char('?') => app.toggle_help(),

                    // Basic vertical navigation
                    KeyCode::Down | KeyCode::Char('j') => app.next(),
                    KeyCode::Up | KeyCode::Char('k') => app.previous(),

                    // Jump to top (Home or 'g')
                    KeyCode::Home | KeyCode::Char('g') => app.first(),
                    // Jump to bottom (End or 'G')
                    KeyCode::End | KeyCode::Char('G') => app.last(),

                    // Page up (PageUp, Ctrl+b, or Ctrl+u)
                    KeyCode::PageUp => app.page_up(),
                    KeyCode::Char('b') | KeyCode::Char('u')
                        if key.modifiers.contains(KeyModifiers::CONTROL) =>
                    {
                        app.page_up()
                    }
                    // Page down (PageDown, Ctrl+f, or Ctrl+d)
                    KeyCode::PageDown => app.page_down(),
                    KeyCode::Char('f') | KeyCode::Char('d')
                        if key.modifiers.contains(KeyModifiers::CONTROL) =>
                    {
                        app.page_down()
                    }

                    // Quit application
                    KeyCode::Esc | KeyCode::Char('q') => {
                        if app.show_help {
                            app.toggle_help();
                        } else {
                            app.quit();
                        }
                    }

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
