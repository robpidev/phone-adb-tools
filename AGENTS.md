# adb-scrcpy-tui

Rust edition 2024, requires Rust 1.85+.

## Commands

- Build: `cargo build`
- Test: `cargo test` (25 unit tests, no integration)
- Lint: `cargo clippy` (warns pedantic, unwrap_used, panic)

## Architecture

| File | Role |
|------|------|
| `src/main.rs` | Entry, event loop, key/mouse dispatch. Extracted fns: `handle_key_event`, `handle_mouse_event`, `handle_edit_mode`, `drain_scrcpy_logs` |
| `src/app.rs` | State machine (`App`), holds `Box<dyn AdbClient>`. Wireless non-blocking via `WirelessState` (polls 100ms, 30 attempts). Helpers: `is_selected_wifi()`, `is_selected_usb()`, `log_ok()`, `log_err()` |
| `src/adb.rs` | `AdbClient` trait + `RealAdbClient`. All fns return `anyhow::Result`. Mock at `adb::mock::MockAdbClient` |
| `src/ui.rs` | ratatui, 5 panel fns. Uses `layout::calculate()` for all rects |
| `src/layout.rs` | Single source of truth for widget positions. `calculate()` must stay in sync with `ui.rs` — mouse hit-testing depends on it |

## Testing

- Mock ADB via `adb::mock::MockAdbClient` (set `devices`, `phone_ips`, `connect_ok`, etc.)
- Tests in `app::tests`, `layout::tests`
- `App::new()` calls `refresh_devices()` — account for initial logs in tests

## Conventions

- UI labels in Spanish, Font Awesome Unicode (`\u{f00c}` check, `\u{f00d}` cross)
- Synchronous event loop, `event::poll(100ms)`
- `start_edit(app, mode, buffer)` helper avoids borrow conflicts with `&app.field`
