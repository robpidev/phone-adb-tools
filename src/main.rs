#![warn(
    clippy::pedantic,
    clippy::unwrap_used,
    clippy::panic,
)]

//! TUI for managing Android devices via ADB and launching scrcpy sessions.

mod adb;
mod app;
mod event;
mod layout;
mod types;
mod ui;

use std::io;

use crossterm::{
    event::{
        DisableMouseCapture, EnableMouseCapture, Event, MouseButton, MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;

use adb::AdbClient;

fn main() -> anyhow::Result<()> {
    let adb_client = Box::new(adb::RealAdbClient);

    if let Err(e) = adb_client.check_adb() {
        eprintln!("Error: {e}");
        eprintln!();
        eprintln!("Install adb:");
        eprintln!("  Debian/Ubuntu: sudo apt install adb");
        eprintln!("  Arch:          sudo pacman -S android-tools");
        eprintln!("  Fedora:        sudo dnf install adb");
        std::process::exit(1);
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = app::App::new(adb_client);

    'main: loop {
        app.frame_count = app.frame_count.wrapping_add(1);
        terminal.draw(|f| ui::ui(f, &app))?;

        event::drain_scrcpy_logs(&mut app);
        app.tick_wireless();

        if crossterm::event::poll(std::time::Duration::from_millis(100))? {
            match crossterm::event::read()? {
                Event::Key(key) if !event::handle_key_event(&mut app, key) => break 'main,
                Event::Mouse(mouse)
                    if mouse.kind == MouseEventKind::Down(MouseButton::Left) =>
                {
                    event::handle_mouse_event(&mut app, mouse, &terminal);
                }
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    Ok(())
}
