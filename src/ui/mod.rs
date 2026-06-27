mod camera;
mod modals;
mod status;
mod widgets;

use ratatui::{
    prelude::*,
    widgets::{Block, List, ListItem, ListState, Paragraph},
};

use crate::app::App;
use crate::layout;
use crate::types::{tr, ConnectionType, EditMode, Panel};

pub fn log_style(text: &str) -> Style {
    if text.starts_with("\u{f00c}") {
        Style::default().fg(Color::Green)
    } else if text.starts_with("\u{f00d}") {
        Style::default().fg(Color::Red)
    } else if text.contains("scrcpy") {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    }
}

fn render_device_list(frame: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = app
        .devices
        .iter()
        .map(|d| {
            let icon = match d.connection {
                ConnectionType::Usb => "\u{f287}",
                ConnectionType::TcpIp => "\u{f1eb}",
            };
            let display = if d.model.is_empty() {
                format!("{icon} {}", d.serial)
            } else {
                format!("{icon} {}", d.model)
            };
            ListItem::new(display)
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.selected));

    let dev_list = List::new(items)
        .block(
            Block::bordered()
                .border_set(symbols::border::ROUNDED)
                .border_style(Style::default().fg(Color::Cyan))
                .title(format!(
                    " {} {} ",
                    "\u{f17b}",
                    tr(app.lang, "Dispositivos", "Devices")
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

    frame.render_stateful_widget(dev_list, area, &mut state);
}

fn render_info_panel(frame: &mut Frame, area: Rect, app: &App) {
    let lang = app.lang;
    let info_block = Block::bordered()
        .border_set(symbols::border::ROUNDED)
        .border_style(Style::default().fg(Color::Cyan))
        .title(format!(
            " {} ",
            tr(lang, "Información", "Information")
        ))
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    let info_lines: Vec<Line> = if let Some(d) = app.selected_info() {
        let model = if d.model.is_empty() {
            tr(lang, "(desconocido)", "(unknown)").to_string()
        } else {
            d.model.clone()
        };
        let ip = match d.connection {
            ConnectionType::TcpIp => d.serial.split(':').next().unwrap_or("").to_string(),
            ConnectionType::Usb => "(USB)".to_string(),
        };
        let os = if d.android_version.is_empty() {
            tr(lang, "(desconocido)", "(unknown)").to_string()
        } else {
            format!("Android {}", d.android_version)
        };
        let mode_icon = match d.connection {
            ConnectionType::Usb => "\u{f287}",
            ConnectionType::TcpIp => "\u{f1eb}",
        };
        let mode = match d.connection {
            ConnectionType::Usb => "USB",
            ConnectionType::TcpIp => "WiFi",
        };
        vec![
            Line::from(format!(" {}: {model}", tr(lang, "Modelo", "Model"))),
            Line::from(format!(" IP:     {ip}")),
            Line::from(format!(" OS:     {os}")),
            Line::from(format!(" {}:   {mode_icon} {mode}", tr(lang, "Modo", "Mode"))),
        ]
    } else {
        vec![Line::from(format!(
            " {}",
            tr(lang, "No hay dispositivo seleccionado", "No device selected")
        ))]
    };

    let info_para = Paragraph::new(info_lines).block(info_block);
    frame.render_widget(info_para, area);
}

fn render_inputs(frame: &mut Frame, ly: &layout::LayoutRects, app: &App) {
    let lang = app.lang;
    let port_value = if app.edit_mode == EditMode::Port {
        &app.edit_buffer
    } else {
        &app.port.to_string()
    };
    widgets::render_input_box(
        frame, ly.input_port, tr(lang, "Puerto", "Port"), port_value,
        app.center_focus == 0, app.edit_mode == EditMode::Port, "",
    );

    let ip_value = if app.edit_mode == EditMode::ManualIp {
        &app.edit_buffer
    } else {
        &app.manual_ip
    };
    widgets::render_input_box(
        frame, ly.input_manualip, tr(lang, "IP manual", "Manual IP"), ip_value,
        app.center_focus == 1, app.edit_mode == EditMode::ManualIp,
        tr(lang, "(vacío)", "(empty)"),
    );

    let bitrate_buffer = if app.edit_mode == EditMode::Bitrate {
        &app.edit_buffer
    } else {
        ""
    };
    widgets::render_preset_block(
        frame, ly.bitrate_group, "Bitrate",
        &[
            ("4M", app.quality.bitrate == "4M"),
            ("8M", app.quality.bitrate == "8M"),
            ("16M", app.quality.bitrate == "16M"),
            ("Custom", false),
        ],
        app.center_focus == 2, bitrate_buffer, app.edit_mode == EditMode::Bitrate,
    );

    let fps_buffer = if app.edit_mode == EditMode::Fps {
        &app.edit_buffer
    } else {
        ""
    };
    widgets::render_preset_block(
        frame, ly.fps_group, "FPS",
        &[
            ("30", app.quality.max_fps == 30),
            ("60", app.quality.max_fps == 60),
            ("Custom", false),
        ],
        app.center_focus == 3, fps_buffer, app.edit_mode == EditMode::Fps,
    );

    let maxsize_buffer = if app.edit_mode == EditMode::MaxSize {
        &app.edit_buffer
    } else {
        ""
    };
    widgets::render_preset_block(
        frame, ly.maxsize_group, tr(lang, "Tamaño", "Size"),
        &[
            ("720", app.quality.max_size == 720),
            ("1080", app.quality.max_size == 1080),
            ("1920", app.quality.max_size == 1920),
            (tr(lang, "Sin límite", "No limit"), app.quality.max_size == 0),
        ],
        app.center_focus == 4, maxsize_buffer, app.edit_mode == EditMode::MaxSize,
    );
}

fn render_actions(frame: &mut Frame, ly: &layout::LayoutRects, app: &App) {
    let lang = app.lang;
    let has_dev = app.has_selected();
    let is_usb = app.is_selected_usb();

    let (label0, icon0, f0) = if has_dev {
        (tr(lang, "Lanzar scrcpy", "Launch scrcpy"), "\u{f17b}", app.center_focus == 5)
    } else {
        ("---", "", false)
    };
    let (label1, icon1, f1) = if is_usb {
        (tr(lang, "Hacer inalámbrico", "Make wireless"), "\u{f287}", app.center_focus == 6)
    } else if has_dev {
        (tr(lang, "Desconectar", "Disconnect"), "\u{f00d}", app.center_focus == 6)
    } else {
        ("---", "", false)
    };

    widgets::render_button(frame, ly.btn_0, label0, icon0, f0);
    widgets::render_button(frame, ly.btn_1, label1, icon1, f1);
    widgets::render_button(frame, ly.btn_2, tr(lang, "WiFi manual", "Manual WiFi"), "\u{f1eb}", app.center_focus == 7);
    widgets::render_button(frame, ly.btn_3, tr(lang, "Cámaras", "Cameras"), "\u{f030}", app.center_focus == 8);
}

pub fn ui(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let ly = layout::calculate(area);

    match app.panel {
        Panel::Main => {
            render_device_list(frame, ly.left_panel, app);
            render_info_panel(frame, ly.info_area, app);
            render_inputs(frame, &ly, app);
            render_actions(frame, &ly, app);
        }
        Panel::Camera => {
            camera::render_camera_list(frame, ly.left_panel, app);
            camera::render_camera_panel(frame, &ly, app);
        }
    }

    status::render_status_bar(frame, &ly, app);

    if app.show_logs {
        modals::render_logs_modal(frame, area, app);
    }

    if app.show_help {
        modals::render_help_popup(frame, area, app);
    }
}
