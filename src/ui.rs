use ratatui::{
    prelude::*,
    widgets::{Block, Clear, List, ListItem, ListState, Paragraph, Wrap},
};

use crate::app::{App, ConnectionType, EditMode, Panel};
use crate::layout;

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

fn render_help_popup(frame: &mut Frame, area: Rect, app: &App) {
    let popup_area = Layout::vertical([
        Constraint::Percentage(15),
        Constraint::Percentage(70),
        Constraint::Percentage(15),
    ])
    .split(area)[1];

    let popup_area = Layout::horizontal([
        Constraint::Percentage(15),
        Constraint::Percentage(70),
        Constraint::Percentage(15),
    ])
    .split(popup_area)[1];

    let adb_status = if app.adb_available {
        "\u{f00c} Instalado"
    } else {
        "\u{f00d} No encontrado"
    };
    let scrcpy_status = if app.scrcpy_available {
        "\u{f00c} Instalado"
    } else {
        "\u{f00d} No encontrado"
    };

    let bitrate_help = if app.quality.bitrate.is_empty() {
        "defecto".to_string()
    } else {
        app.quality.bitrate.clone()
    };
    let max_help = if app.quality.max_size == 0 {
        "sin límite".to_string()
    } else {
        app.quality.max_size.to_string()
    };
    let fps_help = if app.quality.max_fps == 0 {
        "defecto".to_string()
    } else {
        app.quality.max_fps.to_string()
    };

    let text = vec![
        Line::from(vec![Span::styled(
            " \u{f059} Ayuda — Requisitos y Atajos ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            " Requisitos:",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )]),
        Line::from(format!("   adb    {adb_status}")),
        Line::from(format!("   scrcpy {scrcpy_status}")),
        Line::from(""),
        Line::from(vec![Span::styled(
            " Instalación:",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )]),
        Line::from("   Debian/Ubuntu: sudo apt install adb scrcpy"),
        Line::from("   Arch:          sudo pacman -S android-tools scrcpy"),
        Line::from("   Fedora:        sudo dnf install adb scrcpy"),
        Line::from(""),
        Line::from(format!(" Calidad: bitrate {bitrate_help}  max size {max_help}  fps {fps_help}")),
        Line::from(""),
        Line::from(vec![Span::styled(
            " Atajos:",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )]),
        Line::from("   q        Salir"),
        Line::from("   r        Lanzar scrcpy"),
        Line::from("   R        Refrescar dispositivos"),
        Line::from("   j/k      Navegar lista"),
        Line::from("   Tab/l    Siguiente campo"),
        Line::from("   h        Anterior campo"),
        Line::from("   Enter    Editar / Acción"),
        Line::from("   Esc      Cancelar edición"),
        Line::from("   p/s/f/m  Puerto / bitrate / fps / max"),
        Line::from("   d        Desconectar"),
        Line::from("   c        Cámara (tab cámara)"),
        Line::from("   1        Volver a pantalla principal"),
        Line::from("   L        Mostrar/ocultar logs"),
        Line::from("   ?        Mostrar/ocultar ayuda"),
        Line::from(""),
        Line::from(vec![Span::styled(
            " Presione ? o Esc para cerrar ",
            Style::default().fg(Color::DarkGray),
        )]),
    ];

    let popup = Paragraph::new(text)
        .block(
            Block::bordered()
                .border_set(symbols::border::ROUNDED)
                .border_style(Style::default().fg(Color::Yellow))
                .style(Style::default().bg(Color::Black)),
        )
        .alignment(Alignment::Left);

    frame.render_widget(Clear, popup_area);
    frame.render_widget(popup, popup_area);
}

fn render_logs_modal(frame: &mut Frame, area: Rect, app: &App) {
    let popup_area = Layout::vertical([
        Constraint::Percentage(15),
        Constraint::Percentage(70),
        Constraint::Percentage(15),
    ])
    .split(area)[1];

    let popup_area = Layout::horizontal([
        Constraint::Percentage(15),
        Constraint::Percentage(70),
        Constraint::Percentage(15),
    ])
    .split(popup_area)[1];

    let inner_height = popup_area.height.saturating_sub(2);
    let max_lines = inner_height as usize;

    let logs: Vec<Line> = app
        .logs
        .iter()
        .rev()
        .take(max_lines)
        .rev()
        .map(|l| Line::from(vec![Span::styled(l.clone(), log_style(l))]))
        .collect();

    let log_widget = Paragraph::new(logs)
        .block(
            Block::bordered()
                .border_set(symbols::border::ROUNDED)
                .border_style(Style::default().fg(Color::Blue))
                .title(" Logs ")
                .title_style(
                    Style::default()
                        .fg(Color::Blue)
                        .add_modifier(Modifier::BOLD),
                ),
        );

    frame.render_widget(Clear, popup_area);
    frame.render_widget(log_widget, popup_area);
}

fn render_input_box(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    value: &str,
    is_focused: bool,
    is_editing: bool,
    is_empty_ok: bool,
) {
    let border_style = if is_editing {
        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
    } else if is_focused {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let display = if is_editing {
        format!(" {value}_ ")
    } else if value.is_empty() && is_empty_ok {
        " (vacío) ".to_string()
    } else {
        format!(" {value} ")
    };

    let inner = Paragraph::new(display)
        .block(
            Block::bordered()
                .border_set(symbols::border::ROUNDED)
                .border_style(border_style)
                .title(format!(" {title} "))
                .title_style(border_style),
        );
    frame.render_widget(inner, area);
}

fn render_button(frame: &mut Frame, area: Rect, label: &str, icon: &str, is_focused: bool) {
    let style = if is_focused {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };

    let inner = Paragraph::new(format!(" {icon} {label} "))
        .style(style)
        .alignment(Alignment::Center)
        .block(
            Block::bordered()
                .border_set(symbols::border::ROUNDED)
                .border_style(if is_focused {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::DarkGray)
                }),
        );
    frame.render_widget(inner, area);
}

#[allow(clippy::cast_possible_truncation)]
fn render_preset_block(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    presets: &[(&str, bool)],
    is_focused: bool,
    buffer: &str,
    is_edit: bool,
) {
    let border_style = if is_edit {
        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
    } else if is_focused {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::bordered()
        .border_set(symbols::border::ROUNDED)
        .border_style(border_style)
        .title(format!(" {title} "))
        .title_style(border_style);

    if is_edit {
        let display = format!(" {buffer}_ ");
        let para = Paragraph::new(display).block(block);
        frame.render_widget(para, area);
        return;
    }

    frame.render_widget(&block, area);

    let inner = block.inner(area);
    let n = presets.len() as u16;
    if n == 0 || inner.width < n {
        return;
    }
    let btn_w = inner.width / n;

    for (i, (label, active)) in presets.iter().enumerate() {
        let btn_area = Rect {
            x: inner.x + (i as u16) * btn_w,
            y: inner.y,
            width: btn_w,
            height: inner.height,
        };
        let style = if *active {
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        let para = Paragraph::new(format!(" {label} "))
            .style(style)
            .alignment(Alignment::Center);
        frame.render_widget(para, btn_area);
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
                .title(" \u{f17b} Dispositivos ")
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

fn render_camera_list(frame: &mut Frame, area: Rect, app: &App) {
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

    let cam_list = List::new(items)
        .block(
            Block::bordered()
                .border_set(symbols::border::ROUNDED)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" \u{f030} Cámaras ")
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

fn render_info_panel(frame: &mut Frame, area: Rect, app: &App) {
    let info_block = Block::bordered()
        .border_set(symbols::border::ROUNDED)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Información ")
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    let info_lines: Vec<Line> = if let Some(d) = app.selected_info() {
        let model = if d.model.is_empty() {
            "(desconocido)".to_string()
        } else {
            d.model.clone()
        };
        let ip = match d.connection {
            ConnectionType::TcpIp => d.serial.split(':').next().unwrap_or("").to_string(),
            ConnectionType::Usb => "(USB)".to_string(),
        };
        let os = if d.android_version.is_empty() {
            "(desconocido)".to_string()
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
            Line::from(format!(" Modelo: {model}")),
            Line::from(format!(" IP:     {ip}")),
            Line::from(format!(" OS:     {os}")),
            Line::from(format!(" Modo:   {mode_icon} {mode}")),
        ]
    } else {
        vec![Line::from(" No hay dispositivo seleccionado")]
    };

    let info_para = Paragraph::new(info_lines).block(info_block);
    frame.render_widget(info_para, area);
}

fn render_inputs(frame: &mut Frame, ly: &layout::LayoutRects, app: &App) {
    let port_value = if app.edit_mode == EditMode::Port {
        &app.edit_buffer
    } else {
        &app.port.to_string()
    };
    render_input_box(
        frame, ly.input_port, "Puerto", port_value,
        app.center_focus == 0, app.edit_mode == EditMode::Port, false,
    );

    let ip_value = if app.edit_mode == EditMode::ManualIp {
        &app.edit_buffer
    } else {
        &app.manual_ip
    };
    render_input_box(
        frame, ly.input_manualip, "IP manual", ip_value,
        app.center_focus == 1, app.edit_mode == EditMode::ManualIp, true,
    );

    let bitrate_buffer = if app.edit_mode == EditMode::Bitrate {
        &app.edit_buffer
    } else {
        ""
    };
    render_preset_block(
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
    render_preset_block(
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
    render_preset_block(
        frame, ly.maxsize_group, "Max size",
        &[
            ("720", app.quality.max_size == 720),
            ("1080", app.quality.max_size == 1080),
            ("1920", app.quality.max_size == 1920),
            ("Sin límite", app.quality.max_size == 0),
        ],
        app.center_focus == 4, maxsize_buffer, app.edit_mode == EditMode::MaxSize,
    );
}

fn render_actions(frame: &mut Frame, ly: &layout::LayoutRects, app: &App) {
    let has_dev = app.has_selected();
    let is_usb = app.is_selected_usb();

    let (label0, icon0, f0) = if has_dev {
        ("Lanzar scrcpy", "\u{f17b}", app.center_focus == 5)
    } else {
        ("---", "", false)
    };
    let (label1, icon1, f1) = if is_usb {
        ("Hacer inalámbrico", "\u{f287}", app.center_focus == 6)
    } else if has_dev {
        ("Desconectar", "\u{f00d}", app.center_focus == 6)
    } else {
        ("---", "", false)
    };

    render_button(frame, ly.btn_0, label0, icon0, f0);
    render_button(frame, ly.btn_1, label1, icon1, f1);
    render_button(frame, ly.btn_2, "WiFi manual", "\u{f1eb}", app.center_focus == 7);
}

fn render_camera_panel(frame: &mut Frame, ly: &layout::LayoutRects, app: &App) {
    render_info_panel(frame, ly.info_area, app);

    let zoom_value = if app.edit_mode == EditMode::CameraZoom {
        &app.edit_buffer
    } else {
        &app.camera.camera_zoom
    };
    render_input_box(
        frame, ly.cam_zoom, "Zoom", zoom_value,
        app.center_focus == 0, app.edit_mode == EditMode::CameraZoom, false,
    );

    let fps_value = if app.edit_mode == EditMode::CameraFps {
        &app.edit_buffer
    } else {
        &app.camera.camera_fps
    };
    render_input_box(
        frame, ly.cam_fps, "FPS", fps_value,
        app.center_focus == 1, app.edit_mode == EditMode::CameraFps, false,
    );

    render_preset_block(
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
    render_preset_block(
        frame, ly.cam_size, "Tamaño", &preset_refs,
        app.center_focus == 3, "", false,
    );

    let v4l2_value = if app.edit_mode == EditMode::CameraV4l2 {
        &app.edit_buffer
    } else {
        &app.camera.v4l2_sink
    };
    render_input_box(
        frame, ly.cam_v4l2, "V4L2", v4l2_value,
        app.center_focus == 4, app.edit_mode == EditMode::CameraV4l2, false,
    );

    let nowin_icon = if app.camera.no_window { "\u{f204}" } else { "\u{f205}" };
    render_button(
        frame, ly.cam_nowindow, "Sin ventana", nowin_icon,
        app.center_focus == 5,
    );

    render_button(
        frame, ly.cam_launch, "Lanzar cámara", "\u{f030}",
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
                    .title(" Comando ")
                    .title_style(Style::default().fg(Color::DarkGray)),
            );
        frame.render_widget(preview_para, ly.cam_preview);
    }
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
            render_camera_list(frame, ly.left_panel, app);
            render_camera_panel(frame, &ly, app);
        }
    }

    if app.show_logs {
        render_logs_modal(frame, area, app);
    }

    if app.show_help {
        render_help_popup(frame, area, app);
    }
}
