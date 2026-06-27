use ratatui::prelude::*;

pub struct LayoutRects {
    pub left_panel: Rect,
    pub input_port: Rect,
    pub input_manualip: Rect,
    pub bitrate_group: Rect,
    pub fps_group: Rect,
    pub maxsize_group: Rect,
    pub btn_0: Rect,
    pub btn_1: Rect,
    pub btn_2: Rect,
}

pub fn inside(rect: Rect, x: u16, y: u16) -> bool {
    x >= rect.x
        && x < rect.x + rect.width
        && y >= rect.y
        && y < rect.y + rect.height
}

pub fn preset_click(rect: Rect, x: u16, y: u16, n: u16) -> Option<usize> {
    if !inside(rect, x, y) {
        return None;
    }
    let inner_x = rect.x + 1;
    let inner_y = rect.y + 1;
    let inner_w = rect.width.saturating_sub(2);
    if x < inner_x || x >= inner_x + inner_w || y < inner_y || y >= inner_y + rect.height.saturating_sub(2) {
        return None;
    }
    let rel = x - inner_x;
    let btn_w = inner_w / n;
    let idx = rel / btn_w;
    if idx < n { Some(idx as usize) } else { None }
}

pub fn calculate(area: Rect) -> LayoutRects {
    let sections = Layout::vertical([Constraint::Percentage(72), Constraint::Percentage(28)])
        .split(area);
    let top_section = sections[0];

    let panels = Layout::horizontal([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(top_section);
    let left_panel = panels[0];
    let right_panel = panels[1];

    let right_layout =
        Layout::vertical([Constraint::Length(6), Constraint::Min(9), Constraint::Length(3)])
            .split(right_panel);
    let inputs_area = right_layout[1];
    let actions_area = right_layout[2];

    let input_rows = Layout::vertical([
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
    ])
    .split(inputs_area);

    let row0 = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(input_rows[0]);
    let row1 = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(input_rows[1]);

    let btn_w = Layout::horizontal([
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
    ])
    .split(actions_area);

    LayoutRects {
        left_panel,
        input_port: row0[0],
        input_manualip: row0[1],
        bitrate_group: row1[0],
        fps_group: row1[1],
        maxsize_group: input_rows[2],
        btn_0: btn_w[0],
        btn_1: btn_w[1],
        btn_2: btn_w[2],
    }
}
