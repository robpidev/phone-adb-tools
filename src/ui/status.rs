use ratatui::{
    prelude::*,
    widgets::Paragraph,
};

use crate::app::App;
use crate::layout;
use crate::types::WirelessState;

const SPINNER: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

pub fn render_status_bar(frame: &mut Frame, ly: &layout::LayoutRects, app: &App) {
    let bg = Color::Rgb(30, 30, 30);
    let fg = Color::White;
    let sep_fg = Color::Rgb(70, 70, 70);
    let lang_fg = Color::Yellow;
    let load_fg = Color::Cyan;

    let is_loading = app.loading || app.wireless_state != WirelessState::Idle;

    // Background fill
    frame.render_widget(
        Paragraph::new("").style(Style::default().bg(bg)),
        ly.status_bar,
    );

    // Help button
    let help_style = Style::default().bg(bg).fg(fg).add_modifier(Modifier::BOLD);
    frame.render_widget(
        Paragraph::new(" Help ? ").style(help_style),
        ly.status_help,
    );

    // Separator 1
    frame.render_widget(
        Paragraph::new("│").style(Style::default().bg(bg).fg(sep_fg)),
        Rect { x: ly.status_help.x + ly.status_help.width, y: ly.status_help.y, width: 1, height: 1 },
    );

    // Logs button
    let logs_style = Style::default().bg(bg).fg(fg).add_modifier(Modifier::BOLD);
    frame.render_widget(
        Paragraph::new(" Logs L ").style(logs_style),
        ly.status_logs,
    );

    // Separator 2
    frame.render_widget(
        Paragraph::new("│").style(Style::default().bg(bg).fg(sep_fg)),
        Rect { x: ly.status_logs.x + ly.status_logs.width, y: ly.status_logs.y, width: 1, height: 1 },
    );

    // Spinner / loader
    if is_loading {
        let idx = (app.frame_count / 2) as usize % SPINNER.len();
        let spinner = format!(" {} ", SPINNER[idx]);
        let load_style = Style::default().bg(bg).fg(load_fg);
        frame.render_widget(
            Paragraph::new(spinner).style(load_style),
            ly.status_loader,
        );
    }

    // Language toggle
    let lang_label = app.lang.label();
    let lang_style = Style::default().bg(bg).fg(lang_fg).add_modifier(Modifier::BOLD);
    frame.render_widget(
        Paragraph::new(lang_label).style(lang_style),
        ly.status_lang,
    );
}
