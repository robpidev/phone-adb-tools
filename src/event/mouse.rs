use std::io;

use ratatui::prelude::*;

use crate::app::App;
use crate::layout;
use crate::types::{EditMode, Panel};

use super::start_edit;

#[allow(clippy::too_many_lines)]
pub fn handle_mouse_event(
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

    // Status bar clicks (always available regardless of panel)
    if y == size.height.saturating_sub(1) {
        handle_status_mouse(app, &ly, x);
        return;
    }

    match app.panel {
        Panel::Main => handle_main_mouse(app, &ly, x, y),
        Panel::Camera => handle_camera_mouse(app, &ly, x, y),
    }
}

fn handle_status_mouse(app: &mut App, ly: &layout::LayoutRects, x: u16) {
    if layout::inside(ly.status_help, x, ly.status_help.y) {
        app.show_help = !app.show_help;
    } else if layout::inside(ly.status_logs, x, ly.status_logs.y) {
        app.toggle_logs();
    } else if layout::inside(ly.status_lang, x, ly.status_lang.y) {
        app.toggle_lang();
    }
}

fn handle_main_mouse(
    app: &mut App,
    ly: &layout::LayoutRects,
    x: u16,
    y: u16,
) {
    if layout::inside(ly.left_panel, x, y)
        && y > ly.left_panel.y
        && y < ly.left_panel.y + ly.left_panel.height - 1
    {
        let idx = (y - ly.left_panel.y - 1) as usize;
        if idx < app.devices.len() {
            app.edit_mode = EditMode::None;
            app.select_device(idx);
        }
    } else if layout::inside(ly.input_port, x, y) {
        app.center_focus = 0;
        let port = app.port.to_string();
        start_edit(app, EditMode::Port, &port);
    } else if layout::inside(ly.input_manualip, x, y) {
        app.center_focus = 1;
        let ip = app.manual_ip.clone();
        start_edit(app, EditMode::ManualIp, &ip);
    } else if let Some(idx) = layout::preset_click(ly.bitrate_group, x, y, 4) {
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
    } else if let Some(idx) = layout::preset_click(ly.fps_group, x, y, 3) {
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
    } else if let Some(idx) = layout::preset_click(ly.maxsize_group, x, y, 4) {
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
                app.tr("sin límite", "no limit").to_string()
            } else {
                app.quality.max_size.to_string()
            }
        ));
    } else if layout::inside(ly.btn_0, x, y) {
        app.center_focus = 5;
        app.edit_mode = EditMode::None;
        if app.is_selected_wifi() {
            app.launch_scrcpy();
        } else {
            app.connect_selected();
        }
    } else if layout::inside(ly.btn_1, x, y) {
        app.center_focus = 6;
        app.edit_mode = EditMode::None;
        if app.is_selected_usb() {
            app.make_wireless();
        } else {
            app.disconnect_selected();
        }
    } else if layout::inside(ly.btn_2, x, y) {
        app.center_focus = 7;
        app.edit_mode = EditMode::None;
        app.connect_manual();
    } else if layout::inside(ly.btn_3, x, y) {
        app.center_focus = 8;
        app.edit_mode = EditMode::None;
        app.panel = Panel::Camera;
        app.center_focus = 0;
        app.refresh_cameras();
    }
}

fn handle_camera_mouse(
    app: &mut App,
    ly: &layout::LayoutRects,
    x: u16,
    y: u16,
) {
    if layout::inside(ly.left_panel, x, y)
        && y > ly.left_panel.y
        && y < ly.left_panel.y + ly.left_panel.height - 1
    {
        let idx = (y - ly.left_panel.y - 1) as usize;
        if idx < app.camera.available.len() {
            app.edit_mode = EditMode::None;
            app.select_camera(idx);
        }
    } else if layout::inside(ly.cam_zoom, x, y) {
        app.center_focus = 0;
        let zoom = app.camera.camera_zoom.clone();
        start_edit(app, EditMode::CameraZoom, &zoom);
    } else if layout::inside(ly.cam_fps, x, y) {
        app.center_focus = 1;
        let fps = app.camera.camera_fps.clone();
        start_edit(app, EditMode::CameraFps, &fps);
    } else if let Some(idx) = layout::preset_click(ly.cam_codec, x, y, 2) {
        app.center_focus = 2;
        if idx == 0 {
            app.camera.video_codec = "h264".to_string();
        } else {
            app.camera.video_codec = "h265".to_string();
        }
        app.log(format!("Codec: {}", app.camera.video_codec));
    } else if layout::inside(ly.cam_size, x, y) {
        app.center_focus = 3;
        let presets = app.camera.size_presets();
        #[allow(clippy::cast_possible_truncation)]
        let n = presets.len() as u16;
        if let Some(idx) = layout::preset_click(ly.cam_size, x, y, n)
            && idx < presets.len()
        {
            app.select_camera_size(idx);
        }
    } else if layout::inside(ly.cam_v4l2, x, y) {
        app.center_focus = 4;
        let v4l2 = app.camera.v4l2_sink.clone();
        start_edit(app, EditMode::CameraV4l2, &v4l2);
    } else if layout::inside(ly.cam_nowindow, x, y) {
        app.center_focus = 5;
        app.edit_mode = EditMode::None;
        app.toggle_camera_no_window();
    } else if layout::inside(ly.cam_launch, x, y) {
        app.center_focus = 6;
        app.edit_mode = EditMode::None;
        app.launch_camera_scrcpy();
    }
}
