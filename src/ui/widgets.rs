use ratatui::{
    prelude::*,
    widgets::{Block, Paragraph},
};

pub fn render_input_box(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    value: &str,
    is_focused: bool,
    is_editing: bool,
    empty_label: &str,
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
    } else if value.is_empty() && !empty_label.is_empty() {
        format!(" {empty_label} ")
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

pub fn render_button(frame: &mut Frame, area: Rect, label: &str, icon: &str, is_focused: bool) {
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
pub fn render_preset_block(
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
