use ratatui::prelude::*;

/// Pre-computed rectangles for all interactive and structural areas of the UI.
///
/// Computed once per frame by [`calculate`] so that rendering and mouse
/// hit-testing always agree on widget positions.
pub struct LayoutRects {
    // Main panels
    pub left_panel: Rect,
    pub info_area: Rect,
    // Input fields (main panel)
    pub input_port: Rect,
    pub input_manualip: Rect,
    pub bitrate_group: Rect,
    pub fps_group: Rect,
    pub maxsize_group: Rect,
    // Action buttons (main panel)
    pub btn_0: Rect,
    pub btn_1: Rect,
    pub btn_2: Rect,
    pub btn_3: Rect,
    // Camera panel
    pub cam_zoom: Rect,
    pub cam_fps: Rect,
    pub cam_codec: Rect,
    pub cam_size: Rect,
    pub cam_v4l2: Rect,
    pub cam_nowindow: Rect,
    pub cam_launch: Rect,
    pub cam_preview: Rect,
        // Status bar
        pub status_bar: Rect,
        pub status_help: Rect,
        pub status_logs: Rect,
        pub status_lang: Rect,
        pub status_loader: Rect,
}

/// Returns `true` when the point `(x, y)` falls inside `rect`.
pub fn inside(rect: Rect, x: u16, y: u16) -> bool {
    x >= rect.x
        && x < rect.x + rect.width
        && y >= rect.y
        && y < rect.y + rect.height
}

/// Given a preset-group rectangle with `n` buttons, returns the index of the
/// button that was clicked, or `None` if the click was outside the group.
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

/// Build the full set of UI rectangles for the given terminal `area`.
///
/// The main content fills `area` minus the bottom status bar row.
/// Logs and help are shown as modal popups rendered on top.
#[allow(clippy::too_many_lines)]
pub fn calculate(area: Rect) -> LayoutRects {
    let main_area = Rect {
        x: 0,
        y: 0,
        width: area.width,
        height: area.height.saturating_sub(1),
    };
    let status_bar = Rect {
        x: 0,
        y: area.height.saturating_sub(1),
        width: area.width,
        height: 1,
    };

    let panels = Layout::horizontal([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(main_area);
    let left_panel = panels[0];
    let right_panel = panels[1];

    // Right panel: info (fixed 6 rows) + content (remaining)
    let right_layout = Layout::vertical([
        Constraint::Length(6),
        Constraint::Min(6),
    ])
    .split(right_panel);
    let info_area = right_layout[0];
    let content_area = right_layout[1];

    // ── Main panel layout (inputs + actions) ────────────────────────
    let main_parts = Layout::vertical([
        Constraint::Min(6),
        Constraint::Length(3),
    ])
    .split(content_area);

    let input_rows = Layout::vertical([
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
    ])
    .split(main_parts[0]);

    let main_row0 = Layout::horizontal([
        Constraint::Percentage(50),
        Constraint::Percentage(50),
    ])
    .split(input_rows[0]);

    let main_row1 = Layout::horizontal([
        Constraint::Percentage(50),
        Constraint::Percentage(50),
    ])
    .split(input_rows[1]);

    let btn_w = Layout::horizontal([
        Constraint::Percentage(25),
        Constraint::Percentage(25),
        Constraint::Percentage(25),
        Constraint::Percentage(25),
    ])
    .split(main_parts[1]);

    // ── Camera panel layout ─────────────────────────────────────────
    let cam_blocks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Min(1),
    ])
    .split(content_area);

    let cam_upper = Layout::horizontal([
        Constraint::Percentage(33),
        Constraint::Percentage(33),
        Constraint::Percentage(34),
    ])
    .split(cam_blocks[0]);

    let cam_mid = Layout::horizontal([
        Constraint::Percentage(50),
        Constraint::Percentage(50),
    ])
    .split(cam_blocks[2]);

    // ── Status bar sections ─────────────────────────────────────────
    let status_cols = Layout::horizontal([
        Constraint::Length(8),   // " Help ? "
        Constraint::Length(1),   // "│"
        Constraint::Length(8),   // " Logs L "
        Constraint::Length(1),   // "│"
        Constraint::Min(0),      // filler
        Constraint::Length(4),   // " EN "/" ES "
    ])
    .split(status_bar);

    LayoutRects {
        left_panel,
        info_area,
        input_port: main_row0[0],
        input_manualip: main_row0[1],
        bitrate_group: main_row1[0],
        fps_group: main_row1[1],
        maxsize_group: input_rows[2],
        btn_0: btn_w[0],
        btn_1: btn_w[1],
        btn_2: btn_w[2],
        btn_3: btn_w[3],
        cam_zoom: cam_upper[0],
        cam_fps: cam_upper[1],
        cam_codec: cam_upper[2],
        cam_size: cam_blocks[1],
        cam_v4l2: cam_mid[0],
        cam_nowindow: cam_mid[1],
        cam_launch: cam_blocks[3],
        cam_preview: cam_blocks[4],
        status_bar,
        status_help: status_cols[0],
        status_logs: status_cols[2],
        status_lang: status_cols[5],
        status_loader: status_cols[4],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inside() {
        let rect = Rect::new(10, 10, 20, 5);
        assert!(inside(rect, 10, 10));
        assert!(inside(rect, 29, 14));
        assert!(!inside(rect, 9, 10));
        assert!(!inside(rect, 10, 9));
        assert!(!inside(rect, 30, 10));
        assert!(!inside(rect, 10, 15));
    }

    #[test]
    fn test_inside_zero_area() {
        let rect = Rect::new(0, 0, 0, 0);
        assert!(!inside(rect, 0, 0));
    }

    #[test]
    fn test_preset_click_outside() {
        let rect = Rect::new(0, 0, 10, 3);
        assert!(preset_click(rect, 0, 0, 4).is_none());
        assert!(preset_click(rect, 20, 0, 4).is_none());
    }

    #[test]
    fn test_preset_click_inside() {
        let rect = Rect::new(0, 0, 20, 3);
        let idx = preset_click(rect, 2, 1, 4);
        assert!(idx.is_some());
        assert!(idx.unwrap() < 4);
    }

    #[test]
    fn test_preset_click_narrow() {
        let rect = Rect::new(0, 0, 3, 3);
        let idx = preset_click(rect, 0, 0, 4);
        assert!(idx.is_none());
    }

    #[test]
    fn test_calculate() {
        let area = Rect::new(0, 0, 100, 50);
        let ly = calculate(area);
        assert_eq!(ly.left_panel.x, 0);
        assert!(ly.left_panel.width > 0);
        assert!(ly.info_area.width > 0);
        assert!(ly.cam_zoom.width > 0);
        assert!(ly.cam_fps.width > 0);
        assert!(ly.cam_preview.width > 0);
        assert!(ly.input_port.width > 0);
        assert!(ly.btn_0.width > 0);
        assert!(ly.btn_1.x != ly.btn_0.x);
        assert!(ly.btn_2.x != ly.btn_1.x);
        assert!(ly.btn_3.x != ly.btn_2.x);
        assert_eq!(ly.status_bar.height, 1);
        assert!(ly.status_help.width > 0);
        assert!(ly.status_logs.width > 0);
        assert!(ly.status_lang.width > 0);
    }
}
