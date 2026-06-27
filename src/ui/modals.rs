use ratatui::{
    prelude::*,
    widgets::{Block, Clear, Paragraph},
};

use crate::app::App;
use crate::types::tr;

use super::log_style;

#[allow(clippy::too_many_lines)]
pub fn render_help_popup(frame: &mut Frame, area: Rect, app: &App) {
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

    let lang = app.lang;
    let status_str = |ok: bool| -> String {
        if ok {
            format!("\u{f00c} {}", tr(lang, "Instalado", "Installed"))
        } else {
            format!("\u{f00d} {}", tr(lang, "No encontrado", "Not found"))
        }
    };
    let adb_status = status_str(app.adb_available);
    let scrcpy_status = status_str(app.scrcpy_available);
    let v4l2_status = status_str(app.v4l2_available);

    let bitrate_help = if app.quality.bitrate.is_empty() {
        tr(lang, "defecto", "default").to_string()
    } else {
        app.quality.bitrate.clone()
    };
    let max_help = if app.quality.max_size == 0 {
        tr(lang, "sin límite", "no limit").to_string()
    } else {
        app.quality.max_size.to_string()
    };
    let fps_help = if app.quality.max_fps == 0 {
        tr(lang, "defecto", "default").to_string()
    } else {
        app.quality.max_fps.to_string()
    };

    let all_installed = app.adb_available && app.scrcpy_available && app.v4l2_available;

    let mut text = vec![
        Line::from(vec![Span::styled(
            format!(" {} {} ", "\u{f059}", tr(lang, "Ayuda — Requisitos y Atajos", "Help — Requirements & Shortcuts")),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            format!(" {}:", tr(lang, " Requisitos", " Requirements")),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )]),
        Line::from(format!("   adb    {adb_status}")),
        Line::from(format!("   scrcpy {scrcpy_status}")),
        Line::from(format!("   v4l2   {v4l2_status}")),
    ];

    if !all_installed {
        text.push(Line::from(""));
        text.push(Line::from(vec![Span::styled(
            format!(" {}:", tr(lang, " Instalación", " Installation")),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )]));
        text.push(Line::from("   Debian/Ubuntu: sudo apt install adb scrcpy"));
        text.push(Line::from("   Arch:          sudo pacman -S android-tools scrcpy"));
        text.push(Line::from("   Fedora:        sudo dnf install adb scrcpy"));
        text.push(Line::from("   V4L2:          sudo modprobe v4l2loopback"));
    }

    text.push(Line::from(""));
    text.push(Line::from(format!(
        " {}: {} {}  {} {}  {} {}",
        tr(lang, "Calidad", "Quality"),
        tr(lang, "bitrate", "bitrate"),
        bitrate_help,
        tr(lang, "max size", "max size"),
        max_help,
        tr(lang, "fps", "fps"),
        fps_help,
    )));
    text.push(Line::from(""));
    text.push(Line::from(vec![Span::styled(
        format!(" {}:", tr(lang, " Atajos", " Shortcuts")),
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    )]));
    text.push(Line::from(format!("   q        {}", tr(lang, "Salir", "Quit"))));
    text.push(Line::from(format!("   r        {}", tr(lang, "Lanzar scrcpy", "Launch scrcpy"))));
    text.push(Line::from(format!("   R        {}", tr(lang, "Refrescar dispositivos", "Refresh devices"))));
    text.push(Line::from(format!("   j/k      {}", tr(lang, "Navegar lista", "Navigate list"))));
    text.push(Line::from(format!("   Tab/l    {}", tr(lang, "Siguiente campo", "Next field"))));
    text.push(Line::from(format!("   h        {}", tr(lang, "Anterior campo", "Previous field"))));
    text.push(Line::from(format!("   Enter    {}", tr(lang, "Editar / Acción", "Edit / Action"))));
    text.push(Line::from(format!("   Esc      {}", tr(lang, "Cancelar edición", "Cancel edit"))));
    text.push(Line::from(format!("   p/s/f/m  {}", tr(lang, "Puerto / bitrate / fps / max", "Port / bitrate / fps / max"))));
    text.push(Line::from(format!("   d        {}", tr(lang, "Desconectar", "Disconnect"))));
    text.push(Line::from(format!("   c        {}", tr(lang, "Cámara (tab cámara)", "Camera (camera tab)"))));
    text.push(Line::from(format!("   1        {}", tr(lang, "Volver a pantalla principal", "Back to main"))));
    text.push(Line::from(format!("   L        {}", tr(lang, "Mostrar/ocultar logs", "Show/hide logs"))));
    text.push(Line::from(format!("   ?        {}", tr(lang, "Mostrar/ocultar ayuda", "Show/hide help"))));
    text.push(Line::from(""));
    text.push(Line::from(vec![Span::styled(
        format!(" {} ", tr(lang, "Presione ? o Esc para cerrar", "Press ? or Esc to close")),
        Style::default().fg(Color::DarkGray),
    )]));

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

pub fn render_logs_modal(frame: &mut Frame, area: Rect, app: &App) {
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
