use ratatui::{
    prelude::*,
    widgets::{Block, List, ListItem, ListState, Paragraph, Wrap},
};

use crate::app::App;
use crate::layout;
use crate::types::tr;

use super::render_info_panel;

pub fn render_camera_list(frame: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = app
        .camera
        .available
        .iter()
        .map(|c| {
            ListItem::new(format!("{} {} ({})", c.id, c.name, c.max_resolution))
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.camera.selected_camera));

    let lang = app.lang;
    let cam_list = List::new(items)
        .block(
            Block::bordered()
                .border_set(symbols::border::ROUNDED)
                .border_style(Style::default().fg(Color::Cyan))
                .title(format!(
                    " {} {} ",
                    "\u{f030}",
                    tr(lang, "Cámaras", "Cameras")
                ))
                .title_style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
        )
        .highlight_symbol(">> ")
        .highlight_style(
            Style::default()
                .fg(Color::Green)
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_stateful_widget(cam_list, area, &mut state);
}

pub fn render_camera_panel(frame: &mut Frame, ly: &layout::LayoutRects, app: &App) {
    let lang = app.lang;
    render_info_panel(frame, ly.info_area, app);

    let zoom_value = if app.edit_mode == super::EditMode::CameraZoom {
        &app.edit_buffer
    } else {
        &app.camera.camera_zoom
    };
    super::widgets::render_input_box(
        frame, ly.cam_zoom, "Zoom", zoom_value,
        app.center_focus == 0, app.edit_mode == super::EditMode::CameraZoom, "",
    );

    let fps_value = if app.edit_mode == super::EditMode::CameraFps {
        &app.edit_buffer
    } else {
        &app.camera.camera_fps
    };
    super::widgets::render_input_box(
        frame, ly.cam_fps, "FPS", fps_value,
        app.center_focus == 1, app.edit_mode == super::EditMode::CameraFps, "",
    );

    super::widgets::render_preset_block(
        frame, ly.cam_codec, "Codec",
        &[
            ("h264", app.camera.video_codec == "h264"),
            ("h265", app.camera.video_codec == "h265"),
        ],
        app.center_focus == 2, "", false,
    );

    let presets = app.camera.size_presets();
    let preset_refs: Vec<(&str, bool)> = presets.iter().enumerate().map(|(i, p)| {
        (p.as_str(), i == app.camera.selected_size)
    }).collect();
    super::widgets::render_preset_block(
        frame, ly.cam_size, tr(lang, "Tamaño", "Size"), &preset_refs,
        app.center_focus == 3, "", false,
    );

    let v4l2_value = if app.edit_mode == super::EditMode::CameraV4l2 {
        &app.edit_buffer
    } else {
        &app.camera.v4l2_sink
    };
    super::widgets::render_input_box(
        frame, ly.cam_v4l2, "V4L2", v4l2_value,
        app.center_focus == 4, app.edit_mode == super::EditMode::CameraV4l2, "",
    );

    let nowin_icon = if app.camera.no_window { "\u{f204}" } else { "\u{f205}" };
    super::widgets::render_button(
        frame, ly.cam_nowindow, tr(lang, "Sin ventana", "No window"), nowin_icon,
        app.center_focus == 5,
    );

    super::widgets::render_button(
        frame, ly.cam_launch, tr(lang, "Lanzar cámara", "Launch camera"), "\u{f030}",
        app.center_focus == 6,
    );

    let preview = app.camera_command_preview();
    if !preview.is_empty() {
        let preview_para = Paragraph::new(preview)
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(Color::DarkGray))
            .block(
                Block::bordered()
                    .border_set(symbols::border::ROUNDED)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .title(format!(" {} ", tr(lang, "Comando", "Command")))
                    .title_style(Style::default().fg(Color::DarkGray)),
            );
        frame.render_widget(preview_para, ly.cam_preview);
    }
}
