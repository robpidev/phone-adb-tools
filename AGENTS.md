# adb-scrcpy-tui

Rust edition 2024, requires Rust 1.85+.

## Commands

- Build: `cargo build`
- Test: `cargo test` (39 unit tests, no integration)
- Lint: `cargo clippy` (warns pedantic, unwrap_used, panic)

## Architecture

| File | Role |
|------|------|
| `src/main.rs` | Entry point, event loop (~50 lines). Calls `event::handle_key_event`, `event::handle_mouse_event`, `event::drain_scrcpy_logs` |
| `src/event/mod.rs` | Key event dispatch: `handle_key_event`, `handle_main_key`, `handle_camera_key`, `handle_edit_mode`, `drain_scrcpy_logs`, `start_edit` |
| `src/event/mouse.rs` | Mouse event handlers: `handle_mouse_event`, `handle_main_mouse`, `handle_camera_mouse`, `handle_status_mouse` |
| `src/app/mod.rs` | State machine (`App` struct + main impl), helpers, tests |
| `src/app/camera.rs` | Camera methods on `App` (`refresh_cameras`, `launch_camera_scrcpy`, etc.) + `parse_camera_list()` |
| `src/types.rs` | All types/enums: `ConnectionType`, `Panel`, `EditMode`, `WirelessState`, `DeviceInfo`, `ScrcpyQuality`, `CameraInfo`, `CameraSettings`, `Lang` + `tr()` helper |
| `src/adb.rs` | `AdbClient` trait + `RealAdbClient`. All fns return `anyhow::Result`. Mock at `adb::mock::MockAdbClient` |
| `src/ui/mod.rs` | Main render dispatch: `ui()`, `log_style()`, `render_device_list`, `render_info_panel`, `render_inputs`, `render_actions` |
| `src/ui/camera.rs` | Camera panel rendering: `render_camera_list`, `render_camera_panel` |
| `src/ui/modals.rs` | Modal popups: `render_help_popup`, `render_logs_modal` |
| `src/ui/status.rs` | Status bar rendering: `render_status_bar` |
| `src/ui/widgets.rs` | Reusable widgets: `render_input_box`, `render_button`, `render_preset_block` |
| `src/layout.rs` | Single source of truth for widget positions. `calculate()` must stay in sync with UI — mouse hit-testing depends on it. Status bar rects at bottom |

## Testing

- Mock ADB via `adb::mock::MockAdbClient` (set `devices`, `phone_ips`, `connect_ok`, etc.)
- Tests in `app::tests`, `layout::tests`
- `App::new()` calls `refresh_devices()` — account for initial logs in tests

## Conventions

- UI labels bilingual (English default, Spanish toggleable via status bar), Font Awesome Unicode (`\u{f00c}` check, `\u{f00d}` cross)
- Synchronous event loop, `event::poll(100ms)`
- `start_edit(app, mode, buffer)` helper avoids borrow conflicts with `&app.field`
