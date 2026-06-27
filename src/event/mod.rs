mod mouse;

use crossterm::event::KeyCode;

use crate::app::App;
use crate::types::{EditMode, Panel, tr};

pub const FOCUS_MAIN: usize = 9;
pub const FOCUS_CAMERA: usize = 7;

/// Read any available scrcpy stderr lines into the app log.
pub fn drain_scrcpy_logs(app: &mut App) {
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
pub fn handle_key_event(app: &mut App, key: crossterm::event::KeyEvent) -> bool {
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
            8 => {
                app.panel = Panel::Camera;
                app.center_focus = 0;
                app.refresh_cameras();
            }
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
            let lang = app.lang;
            match app.edit_mode {
                EditMode::Port => {
                    if let Ok(port) = app.edit_buffer.trim().parse::<u16>()
                        && port > 0
                    {
                        app.port = port;
                        app.log(format!(
                            "{} {port}",
                            tr(lang, "Puerto cambiado a", "Port changed to")
                        ));
                    }
                }
                EditMode::Bitrate => {
                    app.quality.bitrate = app.edit_buffer.trim().to_string();
                    app.log(format!(
                        "Bitrate: {}",
                        if app.quality.bitrate.is_empty() {
                            tr(lang, "defecto", "default")
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
                    app.log(format!(
                        "{}: {}",
                        tr(lang, "IP manual", "Manual IP"),
                        app.manual_ip
                    ));
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
                        app.log(format!(
                            "{}: {}",
                            tr(lang, "FPS cámara", "Camera FPS"),
                            app.camera.camera_fps
                        ));
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

pub use mouse::handle_mouse_event;
