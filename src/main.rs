#![warn(
    clippy::pedantic,
    clippy::unwrap_used,
    clippy::panic,
)]

//! TUI for managing Android devices via ADB and launching scrcpy sessions.

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

use adb::AdbClient;
use app::{App, EditMode, Panel};
use layout::{inside, preset_click};

const FOCUS_MAIN: usize = 8;
const FOCUS_CAMERA: usize = 7;

fn main() -> anyhow::Result<()> {
    let adb_client = Box::new(adb::RealAdbClient);

    if let Err(e) = adb_client.check_adb() {
        eprintln!("Error: {e}");
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

    let mut app = App::new(adb_client);

    'main: loop {
        terminal.draw(|f| ui::ui(f, &app))?;

        drain_scrcpy_logs(&mut app);
        app.tick_wireless();

        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) if !handle_key_event(&mut app, key) => break 'main,
                Event::Mouse(mouse)
                    if mouse.kind == MouseEventKind::Down(MouseButton::Left) =>
                {
                    handle_mouse_event(&mut app, mouse, &terminal);
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

fn drain_scrcpy_logs(app: &mut App) {
    let lines: Vec<String> = app
        .scrcpy_rx
        .as_ref()
        .map(|rx| {
            let mut lines = Vec::new();
            while let Ok(line) = rx.try_recv() {
                lines.push(line);
            }
            lines
        })
        .unwrap_or_default();
    for line in lines {
        app.log(format!("\u{f1eb} scrcpy: {line}"));
    }
}

/// Returns `false` to signal the caller to exit the event loop.
fn handle_key_event(app: &mut App, key: crossterm::event::KeyEvent) -> bool {
    if app.show_logs {
        if matches!(key.code, KeyCode::Esc | KeyCode::Char('L')) {
            app.show_logs = false;
        }
        return true;
    }

    if app.show_help {
        if matches!(key.code, KeyCode::Esc | KeyCode::Char('?')) {
            app.show_help = false;
        }
        return true;
    }

    if app.edit_mode != EditMode::None {
        handle_edit_mode(app, key);
        return true;
    }

    match key.code {
        KeyCode::Char('q') => return false,
        KeyCode::Char('L') => app.toggle_logs(),
        KeyCode::Char('c') => {
            app.panel = Panel::Camera;
            app.center_focus = 0;
            app.refresh_cameras();
        }
        KeyCode::Char('1') => {
            app.panel = Panel::Main;
            app.center_focus = 0;
        }
        _ => match app.panel {
            Panel::Main => handle_main_key(app, key),
            Panel::Camera => handle_camera_key(app, key),
        },
    }
    true
}

fn handle_main_key(app: &mut App, key: crossterm::event::KeyEvent) {
    match key.code {
        KeyCode::Char('r') => {
            if app.is_selected_wifi() {
                app.launch_scrcpy();
            } else {
                app.connect_selected();
            }
        }
        KeyCode::Char('R') => app.refresh_devices(),
        KeyCode::Tab | KeyCode::Char('l') => {
            app.center_focus = (app.center_focus + 1) % FOCUS_MAIN;
        }
        KeyCode::BackTab | KeyCode::Char('h') => {
            app.center_focus = if app.center_focus == 0 {
                FOCUS_MAIN - 1
            } else {
                app.center_focus - 1
            };
        }
        KeyCode::Enter => match app.center_focus {
            0 => {
                let port = app.port.to_string();
                start_edit(app, EditMode::Port, &port);
            }
            1 => {
                let ip = app.manual_ip.clone();
                start_edit(app, EditMode::ManualIp, &ip);
            }
            2 => app.cycle_bitrate(),
            3 => app.cycle_fps(),
            4 => app.cycle_maxsize(),
            5 => {
                if app.is_selected_wifi() {
                    app.launch_scrcpy();
                } else {
                    app.connect_selected();
                }
            }
            6 => {
                if app.is_selected_usb() {
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
            let port = app.port.to_string();
            start_edit(app, EditMode::Port, &port);
        }
        KeyCode::Char('s') => {
            app.center_focus = 2;
            let bitrate = app.quality.bitrate.clone();
            start_edit(app, EditMode::Bitrate, &bitrate);
        }
        KeyCode::Char('f') => {
            app.center_focus = 3;
            let buffer = if app.quality.max_fps > 0 {
                app.quality.max_fps.to_string()
            } else {
                String::new()
            };
            start_edit(app, EditMode::Fps, &buffer);
        }
        KeyCode::Char('m') => {
            app.center_focus = 4;
            let buffer = if app.quality.max_size > 0 {
                app.quality.max_size.to_string()
            } else {
                String::new()
            };
            start_edit(app, EditMode::MaxSize, &buffer);
        }
        KeyCode::Char('d') => app.disconnect_selected(),
        KeyCode::Char('?') => app.show_help = !app.show_help,
        KeyCode::Up | KeyCode::Char('k') if app.selected > 0 => {
            app.select_device(app.selected - 1);
        }
        KeyCode::Down | KeyCode::Char('j') if app.selected + 1 < app.devices.len() => {
            app.select_device(app.selected + 1);
        }
        _ => {}
    }
}

fn handle_camera_key(app: &mut App, key: crossterm::event::KeyEvent) {
    match key.code {
        KeyCode::Char('r') => app.launch_camera_scrcpy(),
        KeyCode::Char('R') => app.refresh_cameras(),
        KeyCode::Tab | KeyCode::Char('l') => {
            app.center_focus = (app.center_focus + 1) % FOCUS_CAMERA;
        }
        KeyCode::BackTab | KeyCode::Char('h') => {
            app.center_focus = if app.center_focus == 0 {
                FOCUS_CAMERA - 1
            } else {
                app.center_focus - 1
            };
        }
        KeyCode::Enter => match app.center_focus {
            0 => {
                let zoom = app.camera.camera_zoom.clone();
                start_edit(app, EditMode::CameraZoom, &zoom);
            }
            1 => {
                let fps = app.camera.camera_fps.clone();
                start_edit(app, EditMode::CameraFps, &fps);
            }
            2 => app.cycle_camera_codec(),
            3 => {
                let presets = app.camera.size_presets();
                let current = app.camera.selected_size;
                let next = (current + 1) % presets.len();
                app.select_camera_size(next);
            }
            4 => {
                let v4l2 = app.camera.v4l2_sink.clone();
                start_edit(app, EditMode::CameraV4l2, &v4l2);
            }
            5 => app.toggle_camera_no_window(),
            6 => app.launch_camera_scrcpy(),
            _ => {}
        },
        KeyCode::Char('?') => app.show_help = !app.show_help,
        KeyCode::Up | KeyCode::Char('k') if app.camera.selected_camera > 0 => {
            app.select_camera(app.camera.selected_camera - 1);
        }
        KeyCode::Down | KeyCode::Char('j')
            if app.camera.selected_camera + 1 < app.camera.available.len() =>
        {
            app.select_camera(app.camera.selected_camera + 1);
        }
        _ => {}
    }
}

fn start_edit(app: &mut App, mode: EditMode, buffer: &str) {
    app.edit_mode = mode;
    app.edit_buffer = buffer.to_string();
}

fn handle_edit_mode(app: &mut App, key: crossterm::event::KeyEvent) {
    match key.code {
        KeyCode::Char(c) => app.edit_buffer.push(c),
        KeyCode::Backspace => {
            app.edit_buffer.pop();
        }
        KeyCode::Enter => {
            match app.edit_mode {
                EditMode::Port => {
                    if let Ok(port) = app.edit_buffer.trim().parse::<u16>()
                        && port > 0
                    {
                        app.port = port;
                        app.log(format!("Puerto cambiado a {port}"));
                    }
                }
                EditMode::Bitrate => {
                    app.quality.bitrate = app.edit_buffer.trim().to_string();
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
                    if let Ok(fps) = app.edit_buffer.trim().parse::<u16>()
                        && fps > 0
                    {
                        app.quality.max_fps = fps;
                        app.log(format!("FPS: {fps}"));
                    }
                }
                EditMode::MaxSize => {
                    if let Ok(size) = app.edit_buffer.trim().parse::<u16>() {
                        app.quality.max_size = size;
                        app.log(format!("Max size: {size}"));
                    }
                }
                EditMode::ManualIp => {
                    app.manual_ip = app.edit_buffer.trim().to_string();
                    app.log(format!("IP manual: {}", app.manual_ip));
                }
                EditMode::CameraZoom => {
                    let value = app.edit_buffer.trim().to_string();
                    if !value.is_empty() {
                        app.camera.camera_zoom = value;
                        app.log(format!("Zoom: {}", app.camera.camera_zoom));
                    }
                }
                EditMode::CameraFps => {
                    let value = app.edit_buffer.trim().to_string();
                    if !value.is_empty() {
                        app.camera.camera_fps = value;
                        app.log(format!("FPS cámara: {}", app.camera.camera_fps));
                    }
                }
                EditMode::CameraV4l2 => {
                    let value = app.edit_buffer.trim().to_string();
                    if !value.is_empty() {
                        app.camera.v4l2_sink = value;
                        app.log(format!("V4L2 sink: {}", app.camera.v4l2_sink));
                    }
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
}

#[allow(clippy::too_many_lines)]
fn handle_mouse_event(
    app: &mut App,
    mouse: crossterm::event::MouseEvent,
    terminal: &Terminal<CrosstermBackend<io::Stdout>>,
) {
    let Ok(size) = terminal.size() else {
        return;
    };
    let area = Rect {
        x: 0,
        y: 0,
        width: size.width,
        height: size.height,
    };

    if app.show_logs {
        app.show_logs = false;
        return;
    }

    if app.show_help {
        app.show_help = false;
        return;
    }

    let ly = layout::calculate(area);
    let x = mouse.column;
    let y = mouse.row;

    match app.panel {
        Panel::Main => handle_main_mouse(app, &ly, x, y),
        Panel::Camera => handle_camera_mouse(app, &ly, x, y),
    }
}

fn handle_main_mouse(
    app: &mut App,
    ly: &layout::LayoutRects,
    x: u16,
    y: u16,
) {
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
        let port = app.port.to_string();
        start_edit(app, EditMode::Port, &port);
    } else if inside(ly.input_manualip, x, y) {
        app.center_focus = 1;
        let ip = app.manual_ip.clone();
        start_edit(app, EditMode::ManualIp, &ip);
    } else if let Some(idx) = preset_click(ly.bitrate_group, x, y, 4) {
        app.center_focus = 2;
        match idx {
            0 => app.quality.bitrate = "4M".to_string(),
            1 => app.quality.bitrate = "8M".to_string(),
            2 => app.quality.bitrate = "16M".to_string(),
            3 => {
                let bitrate = app.quality.bitrate.clone();
                start_edit(app, EditMode::Bitrate, &bitrate);
            }
            _ => {}
        }
        if idx < 3 {
            app.log(format!("Bitrate: {}", app.quality.bitrate));
        }
    } else if let Some(idx) = preset_click(ly.fps_group, x, y, 3) {
        app.center_focus = 3;
        match idx {
            0 => app.quality.max_fps = 30,
            1 => app.quality.max_fps = 60,
            2 => {
                let buffer = if app.quality.max_fps > 0 {
                    app.quality.max_fps.to_string()
                } else {
                    String::new()
                };
                start_edit(app, EditMode::Fps, &buffer);
            }
            _ => {}
        }
        if idx < 2 {
            app.log(format!("FPS: {}", app.quality.max_fps));
        }
    } else if let Some(idx) = preset_click(ly.maxsize_group, x, y, 4) {
        app.center_focus = 4;
        match idx {
            0 => app.quality.max_size = 720,
            1 => app.quality.max_size = 1080,
            2 => app.quality.max_size = 1920,
            3 => app.quality.max_size = 0,
            _ => {}
        }
        app.log(format!(
            "Max size: {}",
            if app.quality.max_size == 0 {
                "sin límite".to_string()
            } else {
                app.quality.max_size.to_string()
            }
        ));
    } else if inside(ly.btn_0, x, y) {
        app.center_focus = 5;
        app.edit_mode = EditMode::None;
        if app.is_selected_wifi() {
            app.launch_scrcpy();
        } else {
            app.connect_selected();
        }
    } else if inside(ly.btn_1, x, y) {
        app.center_focus = 6;
        app.edit_mode = EditMode::None;
        if app.is_selected_usb() {
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

fn handle_camera_mouse(
    app: &mut App,
    ly: &layout::LayoutRects,
    x: u16,
    y: u16,
) {
    if inside(ly.left_panel, x, y)
        && y > ly.left_panel.y
        && y < ly.left_panel.y + ly.left_panel.height - 1
    {
        let idx = (y - ly.left_panel.y - 1) as usize;
        if idx < app.camera.available.len() {
            app.edit_mode = EditMode::None;
            app.select_camera(idx);
        }
    } else if inside(ly.cam_zoom, x, y) {
        app.center_focus = 0;
        let zoom = app.camera.camera_zoom.clone();
        start_edit(app, EditMode::CameraZoom, &zoom);
    } else if inside(ly.cam_fps, x, y) {
        app.center_focus = 1;
        let fps = app.camera.camera_fps.clone();
        start_edit(app, EditMode::CameraFps, &fps);
    } else if let Some(idx) = preset_click(ly.cam_codec, x, y, 2) {
        app.center_focus = 2;
        if idx == 0 {
            app.camera.video_codec = "h264".to_string();
        } else {
            app.camera.video_codec = "h265".to_string();
        }
        app.log(format!("Codec: {}", app.camera.video_codec));
    } else if inside(ly.cam_size, x, y) {
        app.center_focus = 3;
        let presets = app.camera.size_presets();
        #[allow(clippy::cast_possible_truncation)]
        let n = presets.len() as u16;
        if let Some(idx) = preset_click(ly.cam_size, x, y, n)
            && idx < presets.len()
        {
            app.select_camera_size(idx);
        }
    } else if inside(ly.cam_v4l2, x, y) {
        app.center_focus = 4;
        let v4l2 = app.camera.v4l2_sink.clone();
        start_edit(app, EditMode::CameraV4l2, &v4l2);
    } else if inside(ly.cam_nowindow, x, y) {
        app.center_focus = 5;
        app.edit_mode = EditMode::None;
        app.toggle_camera_no_window();
    } else if inside(ly.cam_launch, x, y) {
        app.center_focus = 6;
        app.edit_mode = EditMode::None;
        app.launch_camera_scrcpy();
    }
}
