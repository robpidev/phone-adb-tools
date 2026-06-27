use ratatui::{prelude::*, widgets::*};

use crate::adb;
use crate::app::{App, ConnectionType, EditMode};

fn log_style(text: &str) -> Style {
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

    let adb_ok = adb::check_adb();
    let scrcpy_ok = std::process::Command::new("scrcpy")
        .arg("--version")
        .output()
        .is_ok();
    let adb_status = if adb_ok {
        "\u{f00c} Instalado"
    } else {
        "\u{f00d} No encontrado"
    };
    let scrcpy_status = if scrcpy_ok {
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
        format!(" {}_ ", value)
    } else if value.is_empty() && is_empty_ok {
        " (vacío) ".to_string()
    } else {
        format!(" {} ", value)
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
        let display = format!(" {}_ ", buffer);
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
        let para = Paragraph::new(format!(" {} ", label))
            .style(style)
            .alignment(Alignment::Center);
        frame.render_widget(para, btn_area);
    }
}

pub fn ui(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let sections = Layout::vertical([Constraint::Percentage(72), Constraint::Percentage(28)])
        .split(area);
    let top_section = sections[0];
    let logs_section = sections[1];

    let panels = Layout::horizontal([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(top_section);
    let left_panel = panels[0];
    let right_panel = panels[1];

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

    frame.render_stateful_widget(dev_list, left_panel, &mut state);

    let right_layout =
        Layout::vertical([Constraint::Length(6), Constraint::Min(6), Constraint::Length(3)])
            .split(right_panel);
    let info_area = right_layout[0];
    let inputs_area = right_layout[1];
    let actions_area = right_layout[2];

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
    frame.render_widget(info_para, info_area);

    let input_rows = Layout::vertical([
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
    ])
    .split(inputs_area);

    let row0 = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(input_rows[0]);

    let port_value = if app.edit_mode == EditMode::Port {
        &app.edit_buffer
    } else {
        &app.port.to_string()
    };
    render_input_box(
        frame, row0[0], "Puerto", port_value,
        app.center_focus == 0, app.edit_mode == EditMode::Port, false,
    );

    let ip_value = if app.edit_mode == EditMode::ManualIp {
        &app.edit_buffer
    } else {
        &app.manual_ip
    };
    render_input_box(
        frame, row0[1], "IP manual", ip_value,
        app.center_focus == 1, app.edit_mode == EditMode::ManualIp, true,
    );

    let row1 = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(input_rows[1]);

    let bitrate_buffer = if app.edit_mode == EditMode::Bitrate {
        &app.edit_buffer
    } else {
        ""
    };
    render_preset_block(
        frame, row1[0], "Bitrate",
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
        frame, row1[1], "FPS",
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
        frame, input_rows[2], "Max size",
        &[
            ("720", app.quality.max_size == 720),
            ("1080", app.quality.max_size == 1080),
            ("1920", app.quality.max_size == 1920),
            ("Sin límite", app.quality.max_size == 0),
        ],
        app.center_focus == 4, maxsize_buffer, app.edit_mode == EditMode::MaxSize,
    );

    let btn_w = Layout::horizontal([
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
    ])
    .split(actions_area);

    let has_dev = app.selected_info().is_some();
    let is_usb = app
        .selected_info()
        .map(|d| d.connection == ConnectionType::Usb)
        .unwrap_or(false);

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

    render_button(frame, btn_w[0], label0, icon0, f0);
    render_button(frame, btn_w[1], label1, icon1, f1);
    render_button(frame, btn_w[2], "WiFi manual", "\u{f1eb}", app.center_focus == 7);

    let logs: Vec<Line> = app
        .logs
        .iter()
        .rev()
        .take(12)
        .rev()
        .map(|l| Line::from(vec![Span::styled(l.clone(), log_style(l))]))
        .collect();

    let log_widget = Paragraph::new(logs).block(
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

    frame.render_widget(log_widget, logs_section);

    if app.show_help {
        render_help_popup(frame, area, app);
    }
}
