mod adb;
mod app;
mod layout;
mod ui;

use std::{io, time::Duration};

use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, MouseButton,
        MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;

use app::{App, ConnectionType, EditMode};
use layout::{inside, preset_click};

const FOCUS_ITEMS: usize = 8;

fn main() -> anyhow::Result<()> {
    if !adb::check_adb() {
        eprintln!("Error: adb no encontrado en PATH");
        eprintln!();
        eprintln!("Instala adb:");
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

    let mut app = App {
        devices: Vec::new(),
        selected: 0,
        logs: Vec::new(),
        last_refresh: None,
        port: 5555,
        manual_ip: String::new(),
        edit_mode: EditMode::None,
        edit_buffer: String::new(),
        center_focus: 0,
        quality: app::ScrcpyQuality::default(),
        show_help: false,
        scrcpy_rx: None,
    };

    app.refresh_devices();

    loop {
        terminal.draw(|f| ui::ui(f, &app))?;

        let scrcpy_lines: Vec<String> = app.scrcpy_rx.as_ref()
            .map(|rx| {
                let mut lines = Vec::new();
                while let Ok(line) = rx.try_recv() {
                    lines.push(line);
                }
                lines
            })
            .unwrap_or_default();
        for line in scrcpy_lines {
            app.log(format!("\u{f1eb} scrcpy: {line}"));
        }

        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => {
                    if app.show_help {
                        match key.code {
                            KeyCode::Esc | KeyCode::Char('?') => app.show_help = false,
                            _ => {}
                        }
                        continue;
                    }

                    if app.edit_mode != EditMode::None {
                        match key.code {
                            KeyCode::Char(c) => {
                                app.edit_buffer.push(c);
                            }
                            KeyCode::Backspace => {
                                app.edit_buffer.pop();
                            }
                            KeyCode::Enter => {
                                match app.edit_mode {
                                    EditMode::Port => {
                                        if let Ok(port) =
                                            app.edit_buffer.trim().parse::<u16>() && port > 0
                                        {
                                            app.port = port;
                                            app.log(format!("Puerto cambiado a {port}"));
                                        }
                                    }
                                    EditMode::Bitrate => {
                                        app.quality.bitrate =
                                            app.edit_buffer.trim().to_string();
                                        app.log(format!(
                                            "Bitrate: {}",
                                            if app.quality.bitrate.is_empty() {
                                                "defecto"
                                            } else {
                                                &app.quality.bitrate
                                            }
                                        ));
                                    }
                                    EditMode::Fps => {
                                        if let Ok(fps) =
                                            app.edit_buffer.trim().parse::<u16>()
                                            && fps > 0
                                        {
                                            app.quality.max_fps = fps;
                                            app.log(format!("FPS: {fps}"));
                                        }
                                    }
                                    EditMode::MaxSize => {
                                        if let Ok(size) =
                                            app.edit_buffer.trim().parse::<u16>()
                                        {
                                            app.quality.max_size = size;
                                            app.log(format!(
                                                "Max size: {}",
                                                app.quality.max_size
                                            ));
                                        }
                                    }
                                    EditMode::ManualIp => {
                                        app.manual_ip =
                                            app.edit_buffer.trim().to_string();
                                        app.log(format!(
                                            "IP manual: {}",
                                            app.manual_ip
                                        ));
                                    }
                                    EditMode::None => {}
                                }
                                app.edit_mode = EditMode::None;
                                app.edit_buffer.clear();
                            }
                            KeyCode::Esc => {
                                app.edit_mode = EditMode::None;
                                app.edit_buffer.clear();
                            }
                            _ => {}
                        }
                        continue;
                    }

                    match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Char('r') => {
                            let is_wifi = app
                                .selected_info()
                                .map(|d| d.connection == ConnectionType::TcpIp)
                                .unwrap_or(false);
                            if is_wifi {
                                app.launch_scrcpy();
                            } else {
                                app.connect_selected();
                            }
                        }
                        KeyCode::Char('R') => app.refresh_devices(),
                        KeyCode::Tab | KeyCode::Char('l') => {
                            app.center_focus =
                                (app.center_focus + 1) % FOCUS_ITEMS;
                        }
                        KeyCode::BackTab | KeyCode::Char('h') => {
                            app.center_focus = if app.center_focus == 0 {
                                FOCUS_ITEMS - 1
                            } else {
                                app.center_focus - 1
                            };
                        }
                        KeyCode::Enter => match app.center_focus {
                            0 => {
                                app.edit_mode = EditMode::Port;
                                app.edit_buffer = app.port.to_string();
                            }
                            1 => {
                                app.edit_mode = EditMode::ManualIp;
                                app.edit_buffer = app.manual_ip.clone();
                            }
                            2 => app.cycle_bitrate(),
                            3 => app.cycle_fps(),
                            4 => app.cycle_maxsize(),
                            5 => {
                                let is_wifi = app
                                    .selected_info()
                                    .map(|d| d.connection == ConnectionType::TcpIp)
                                    .unwrap_or(false);
                                if is_wifi {
                                    app.launch_scrcpy();
                                } else {
                                    app.connect_selected();
                                }
                            }
                            6 => {
                                let is_usb = app
                                    .selected_info()
                                    .map(|d| d.connection == ConnectionType::Usb)
                                    .unwrap_or(false);
                                if is_usb {
                                    app.make_wireless();
                                } else {
                                    app.disconnect_selected();
                                }
                            }
                            7 => app.connect_manual(),
                            _ => {}
                        },
                        KeyCode::Char('p') => {
                            app.center_focus = 0;
                            app.edit_mode = EditMode::Port;
                            app.edit_buffer = app.port.to_string();
                        }
                        KeyCode::Char('s') => {
                            app.center_focus = 2;
                            app.edit_mode = EditMode::Bitrate;
                            app.edit_buffer = app.quality.bitrate.clone();
                        }
                        KeyCode::Char('f') => {
                            app.center_focus = 3;
                            app.edit_mode = EditMode::Fps;
                            app.edit_buffer = if app.quality.max_fps > 0 {
                                app.quality.max_fps.to_string()
                            } else {
                                String::new()
                            };
                        }
                        KeyCode::Char('m') => {
                            app.center_focus = 4;
                            app.edit_mode = EditMode::MaxSize;
                            app.edit_buffer = if app.quality.max_size > 0 {
                                app.quality.max_size.to_string()
                            } else {
                                String::new()
                            };
                        }
                        KeyCode::Char('d') => app.disconnect_selected(),
                        KeyCode::Char('?') => {
                            app.show_help = !app.show_help;
                        }
                        KeyCode::Up | KeyCode::Char('k')
                            if app.selected > 0 =>
                        {
                            app.select_device(app.selected - 1);
                        }
                        KeyCode::Down | KeyCode::Char('j')
                            if app.selected + 1 < app.devices.len() =>
                        {
                            app.select_device(app.selected + 1);
                        }
                        _ => {}
                    }
                }
                Event::Mouse(mouse)
                    if mouse.kind == MouseEventKind::Down(MouseButton::Left) =>
                {
                    let size = terminal.size()?;
                    let area = Rect {
                        x: 0,
                        y: 0,
                        width: size.width,
                        height: size.height,
                    };

                    if app.show_help {
                        app.show_help = false;
                        continue;
                    }

                    let ly = layout::calculate(area);
                    let x = mouse.column;
                    let y = mouse.row;

                    if inside(ly.left_panel, x, y)
                        && y > ly.left_panel.y
                        && y < ly.left_panel.y + ly.left_panel.height - 1
                    {
                        let idx = (y - ly.left_panel.y - 1) as usize;
                        if idx < app.devices.len() {
                            app.edit_mode = EditMode::None;
                            app.select_device(idx);
                        }
                    } else if inside(ly.input_port, x, y) {
                        app.center_focus = 0;
                        app.edit_mode = EditMode::None;
                        app.edit_mode = EditMode::Port;
                        app.edit_buffer = app.port.to_string();
                    } else if inside(ly.input_manualip, x, y) {
                        app.center_focus = 1;
                        app.edit_mode = EditMode::None;
                        app.edit_mode = EditMode::ManualIp;
                        app.edit_buffer = app.manual_ip.clone();
                    } else if let Some(idx) = preset_click(ly.bitrate_group, x, y, 4) {
                        app.center_focus = 2;
                        app.edit_mode = EditMode::None;
                        match idx {
                            0 => app.quality.bitrate = "4M".to_string(),
                            1 => app.quality.bitrate = "8M".to_string(),
                            2 => app.quality.bitrate = "16M".to_string(),
                            3 => {
                                app.edit_mode = EditMode::Bitrate;
                                app.edit_buffer = app.quality.bitrate.clone();
                            }
                            _ => {}
                        }
                        app.log(format!("Bitrate: {}", app.quality.bitrate));
                    } else if let Some(idx) = preset_click(ly.fps_group, x, y, 3) {
                        app.center_focus = 3;
                        app.edit_mode = EditMode::None;
                        match idx {
                            0 => app.quality.max_fps = 30,
                            1 => app.quality.max_fps = 60,
                            2 => {
                                app.edit_mode = EditMode::Fps;
                                app.edit_buffer = if app.quality.max_fps > 0 {
                                    app.quality.max_fps.to_string()
                                } else {
                                    String::new()
                                };
                            }
                            _ => {}
                        }
                        if idx < 2 {
                            app.log(format!("FPS: {}", app.quality.max_fps));
                        }
                    } else if let Some(idx) = preset_click(ly.maxsize_group, x, y, 4) {
                        app.center_focus = 4;
                        app.edit_mode = EditMode::None;
                        match idx {
                            0 => app.quality.max_size = 720,
                            1 => app.quality.max_size = 1080,
                            2 => app.quality.max_size = 1920,
                            3 => app.quality.max_size = 0,
                            _ => {}
                        }
                        app.log(format!(
                            "Max size: {}",
                            if app.quality.max_size == 0 { "sin límite".to_string() } else { app.quality.max_size.to_string() }
                        ));
                    } else if inside(ly.btn_0, x, y) {
                        app.center_focus = 5;
                        app.edit_mode = EditMode::None;
                        let is_wifi = app
                            .selected_info()
                            .map(|d| d.connection == ConnectionType::TcpIp)
                            .unwrap_or(false);
                        if is_wifi {
                            app.launch_scrcpy();
                        } else {
                            app.connect_selected();
                        }
                    } else if inside(ly.btn_1, x, y) {
                        app.center_focus = 6;
                        app.edit_mode = EditMode::None;
                        let is_usb = app
                            .selected_info()
                            .map(|d| d.connection == ConnectionType::Usb)
                            .unwrap_or(false);
                        if is_usb {
                            app.make_wireless();
                        } else {
                            app.disconnect_selected();
                        }
                    } else if inside(ly.btn_2, x, y) {
                        app.center_focus = 7;
                        app.edit_mode = EditMode::None;
                        app.connect_manual();
                    }
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
