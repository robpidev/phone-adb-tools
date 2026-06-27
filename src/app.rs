use std::sync::mpsc;
use std::time::Instant;

use crate::adb::{self, AdbClient};

/// How a device is connected to the host.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectionType {
    Usb,
    TcpIp,
}

/// Current active panel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Panel {
    Main,
    Camera,
}

/// Which input field is currently being edited.
#[derive(Clone, Copy, PartialEq)]
pub enum EditMode {
    None,
    Port,
    Bitrate,
    Fps,
    MaxSize,
    ManualIp,
    CameraZoom,
    CameraFps,
    CameraV4l2,
}

/// State machine for the non-blocking wireless switch.
#[derive(Debug, Clone, PartialEq)]
pub enum WirelessState {
    Idle,
    WaitingTcpIp {
        serial: String,
        attempts: u8,
        launch_scrcpy: bool,
    },
}

/// Information about a connected Android device.
#[derive(Clone)]
pub struct DeviceInfo {
    pub serial: String,
    pub model: String,
    pub android_version: String,
    pub connection: ConnectionType,
}

/// Configurable scrcpy quality settings.
#[derive(Clone, Default)]
pub struct ScrcpyQuality {
    pub bitrate: String,
    pub max_size: u16,
    pub max_fps: u16,
}

/// Camera information parsed from `scrcpy --list-cameras`.
#[derive(Clone)]
pub struct CameraInfo {
    pub id: String,
    pub name: String,
    pub max_resolution: String,
}

/// Camera-specific scrcpy settings.
#[derive(Clone)]
pub struct CameraSettings {
    pub available: Vec<CameraInfo>,
    pub selected_camera: usize,
    pub selected_size: usize,
    pub camera_zoom: String,
    pub camera_fps: String,
    pub video_codec: String,
    pub v4l2_sink: String,
    pub no_window: bool,
}

impl CameraSettings {
    pub fn selected_camera_info(&self) -> Option<&CameraInfo> {
        self.available.get(self.selected_camera)
    }

    pub fn current_size(&self) -> String {
        let presets = self.size_presets();
        presets
            .get(self.selected_size)
            .cloned()
            .unwrap_or_else(|| "640x480".to_string())
    }

    pub fn size_presets(&self) -> Vec<String> {
        let mut presets = vec![
            "640x480".to_string(),
            "1280x720".to_string(),
            "1920x1080".to_string(),
        ];
        if let Some(cam) = self.selected_camera_info()
            && !presets.contains(&cam.max_resolution)
        {
            presets.push(cam.max_resolution.clone());
        }
        presets
    }
}

/// Main application state and logic.
pub struct App {
    pub(crate) devices: Vec<DeviceInfo>,
    pub(crate) selected: usize,
    pub(crate) logs: Vec<String>,
    pub(crate) last_refresh: Option<Instant>,
    pub port: u16,
    pub manual_ip: String,
    pub edit_mode: EditMode,
    pub edit_buffer: String,
    pub center_focus: usize,
    pub quality: ScrcpyQuality,
    pub show_help: bool,
    pub wireless_state: WirelessState,
    pub scrcpy_rx: Option<mpsc::Receiver<String>>,
    pub panel: Panel,
    pub camera: CameraSettings,
    pub show_logs: bool,
    pub adb_available: bool,
    pub scrcpy_available: bool,
    adb_client: Box<dyn AdbClient>,
}

impl App {
    /// Create a new app, immediately refreshing the device list.
    pub fn new(adb_client: Box<dyn AdbClient>) -> Self {
        let mut app = Self {
            devices: Vec::new(),
            selected: 0,
            logs: Vec::new(),
            last_refresh: None,
            port: 5555,
            manual_ip: String::new(),
            edit_mode: EditMode::None,
            edit_buffer: String::new(),
            center_focus: 0,
            quality: ScrcpyQuality::default(),
            show_help: false,
            wireless_state: WirelessState::Idle,
            scrcpy_rx: None,
            panel: Panel::Main,
            camera: CameraSettings {
                available: Vec::new(),
                selected_camera: 0,
                selected_size: 0,
                camera_zoom: "1".to_string(),
                camera_fps: "30".to_string(),
                video_codec: "h264".to_string(),
                v4l2_sink: "/dev/video0".to_string(),
                no_window: false,
            },
            show_logs: false,
            adb_available: false,
            scrcpy_available: false,
            adb_client,
        };
        app.refresh_devices();
        app.refresh_tool_checks();
        app
    }

    /// Append a log message (capped at 100 entries) and update status.
    pub fn log<S: Into<String>>(&mut self, msg: S) {
        self.logs.push(msg.into());
        if self.logs.len() > 100 {
            self.logs.remove(0);
        }
    }

    pub fn log_ok(&mut self, msg: impl Into<String>) {
        self.log(format!("\u{f00c} {}", msg.into()));
    }

    pub fn log_err(&mut self, msg: impl Into<String>) {
        self.log(format!("\u{f00d} {}", msg.into()));
    }

    pub fn is_selected_wifi(&self) -> bool {
        self.selected_info().is_some_and(|d| d.connection == ConnectionType::TcpIp)
    }

    pub fn is_selected_usb(&self) -> bool {
        self.selected_info().is_some_and(|d| d.connection == ConnectionType::Usb)
    }

    pub fn has_selected(&self) -> bool {
        self.selected_info().is_some()
    }

    pub fn selected_info(&self) -> Option<&DeviceInfo> {
        self.devices.get(self.selected)
    }

    pub fn refresh_devices(&mut self) {
        self.log("\u{f021} Refrescando dispositivos...");
        let serials = match self.adb_client.list_devices() {
            Ok(s) => s,
            Err(e) => {
                self.log_err(format!("Error listando dispositivos: {e}"));
                return;
            }
        };

        let mut devices = Vec::with_capacity(serials.len());

        for serial in &serials {
            let connection = if adb::is_tcpip(serial) {
                ConnectionType::TcpIp
            } else {
                ConnectionType::Usb
            };
            let (model, version) = self
                .adb_client
                .fetch_device_props(serial)
                .unwrap_or_default();
            devices.push(DeviceInfo {
                serial: serial.clone(),
                model,
                android_version: version,
                connection,
            });
        }

        self.devices = devices;

        if self.selected >= self.devices.len() {
            self.selected = 0;
        }

        self.last_refresh = Some(Instant::now());
        self.refresh_tool_checks();
        self.log(format!("Encontrados {} dispositivos", self.devices.len()));
    }

    pub fn connect_selected(&mut self) {
        if self.is_selected_wifi() {
            self.launch_scrcpy();
            return;
        }
        self.start_wireless(true);
    }

    /// Start wireless switching (non-blocking, for the "Hacer inalámbrico" button).
    pub fn make_wireless(&mut self) {
        self.start_wireless(false);
    }

    fn start_wireless(&mut self, launch_scrcpy: bool) {
        if self.wireless_state != WirelessState::Idle {
            return;
        }

        let Some(serial) = self.selected_info().map(|d| d.serial.clone()) else {
            self.log_err("No hay dispositivo seleccionado");
            return;
        };

        if adb::is_tcpip(&serial) {
            self.log("El dispositivo ya está en modo WiFi");
            return;
        }

        self.log(format!("Activando TCP/IP en {serial}"));
        match self.adb_client.set_tcpip(&serial, self.port) {
            Ok(()) => self.log_ok("TCP/IP activado"),
            Err(e) => {
                self.log_err(format!("Error activando TCP/IP: {e}"));
                return;
            }
        }

        self.wireless_state = WirelessState::WaitingTcpIp {
            serial,
            attempts: 0,
            launch_scrcpy,
        };
    }

    /// Poll the wireless state machine. Call this from the event loop.
    pub fn tick_wireless(&mut self) {
        let (serial, attempts, launch_scrcpy) = match &self.wireless_state {
            WirelessState::Idle => return,
            WirelessState::WaitingTcpIp { serial, attempts, launch_scrcpy } => {
                (serial.clone(), *attempts, *launch_scrcpy)
            }
        };

        if attempts >= 30 {
            self.log_err(format!("No se pudo obtener IP de {serial}"));
            self.wireless_state = WirelessState::Idle;
            return;
        }

        self.wireless_state = WirelessState::WaitingTcpIp {
            serial: serial.clone(),
            attempts: attempts + 1,
            launch_scrcpy,
        };

        if let Ok(Some(ip)) = self.adb_client.get_phone_ip(&serial) {
            self.log_ok(format!("IP detectada: {ip}"));
            let target = format!("{ip}:{}", self.port);
            match self.adb_client.connect_to(&target) {
                Ok(()) => {
                    self.log_ok("Conectado por WiFi");
                    self.refresh_devices();
                    self.wireless_state = WirelessState::Idle;
                    if launch_scrcpy {
                        self.launch_scrcpy();
                    }
                }
                Err(e) => {
                    self.log_err(format!("Error conectando por WiFi: {e}"));
                    self.wireless_state = WirelessState::Idle;
                }
            }
        }
    }

    pub fn launch_scrcpy(&mut self) {
        let Some(serial) = self.selected_info().map(|d| d.serial.clone()) else {
            self.log_err("No hay dispositivo seleccionado");
            return;
        };

        self.log("Lanzando scrcpy...");
        let mut cmd = std::process::Command::new("scrcpy");
        cmd.args(["-s", &serial]);
        if !self.quality.bitrate.is_empty() {
            cmd.args(["--video-bit-rate", &self.quality.bitrate]);
        }
        if self.quality.max_size > 0 {
            cmd.args(["--max-size", &self.quality.max_size.to_string()]);
        }
        if self.quality.max_fps > 0 {
            cmd.args(["--max-fps", &self.quality.max_fps.to_string()]);
        }
        cmd.stdin(std::process::Stdio::null());
        cmd.stdout(std::process::Stdio::null());
        cmd.stderr(std::process::Stdio::piped());

        match cmd.spawn() {
            Ok(mut child) => {
                let Some(stderr) = child.stderr.take() else {
                    self.log_err("No se pudo capturar stderr de scrcpy");
                    return;
                };
                let (tx, rx) = mpsc::channel();
                self.scrcpy_rx = Some(rx);

                std::thread::spawn(move || {
                    use std::io::BufRead;
                    let reader = std::io::BufReader::new(stderr);
                    for line in reader.lines() {
                        match line {
                            Ok(msg) => {
                                if tx.send(msg).is_err() {
                                    break;
                                }
                            }
                            Err(_) => break,
                        }
                    }
                    let _ = child.wait();
                });

                self.log_ok("scrcpy lanzado");
            }
            Err(e) => self.log_err(format!("Error lanzando scrcpy: {e}")),
        }
    }

    pub fn connect_manual(&mut self) {
        let target = self.manual_ip.trim().to_string();
        if target.is_empty() {
            self.log_err("No se ingresó dirección");
            return;
        }

        let full_target = if target.contains(':') {
            target
        } else {
            format!("{}:{}", target, self.port)
        };

        self.log(format!("Conectando a {full_target}..."));
        match self.adb_client.connect_to(&full_target) {
            Ok(()) => {
                self.log_ok("Conectado");
                self.refresh_devices();
            }
            Err(e) => self.log_err(format!("Error conectando: {e}")),
        }
    }

    pub fn cycle_bitrate(&mut self) {
        const PRESETS: [&str; 3] = ["4M", "8M", "16M"];
        let current = self.quality.bitrate.as_str();
        let pos = PRESETS.iter().position(|&p| p == current);
        let next = match pos {
            Some(p) if p + 1 < PRESETS.len() => p + 1,
            _ => 0,
        };
        self.quality.bitrate = PRESETS[next].to_string();
        self.log(format!("Bitrate: {}", self.quality.bitrate));
    }

    pub fn cycle_maxsize(&mut self) {
        const PRESETS: [u16; 4] = [720, 1080, 1920, 0];
        let pos = PRESETS
            .iter()
            .position(|&p| p == self.quality.max_size)
            .unwrap_or(3);
        let next = (pos + 1) % PRESETS.len();
        self.quality.max_size = PRESETS[next];
        self.log(format!(
            "Max size: {}",
            if self.quality.max_size == 0 {
                "sin límite".to_string()
            } else {
                self.quality.max_size.to_string()
            }
        ));
    }

    pub fn cycle_fps(&mut self) {
        const PRESETS: [u16; 2] = [30, 60];
        let current = self.quality.max_fps;
        let pos = PRESETS.iter().position(|&p| p == current);
        let next = match pos {
            Some(p) if p + 1 < PRESETS.len() => p + 1,
            _ => 0,
        };
        self.quality.max_fps = PRESETS[next];
        self.log(format!("FPS: {}", self.quality.max_fps));
    }

    pub fn select_device(&mut self, idx: usize) {
        if idx < self.devices.len() {
            self.selected = idx;
            if let Some(d) = self.devices.get(idx) {
                if d.connection == ConnectionType::TcpIp {
                    self.manual_ip = d.serial.split(':').next().unwrap_or("").to_string();
                } else {
                    self.manual_ip.clear();
                }
            }
        }
    }

    pub fn disconnect_selected(&mut self) {
        let Some(serial) = self.selected_info().map(|d| d.serial.clone()) else {
            self.log_err("No hay dispositivo seleccionado");
            return;
        };

        self.log(format!("Desconectando {serial}..."));
        match self.adb_client.disconnect_from(&serial) {
            Ok(()) => {
                self.log_ok("Desconectado");
                self.refresh_devices();
            }
            Err(e) => self.log_err(format!("Error desconectando {serial}: {e}")),
        }
    }

    /// Refresh cached tool availability checks.
    pub fn refresh_tool_checks(&mut self) {
        self.adb_available = self.adb_client.check_adb().is_ok();
        self.scrcpy_available = check_scrcpy();
    }

    // ── Camera methods ──────────────────────────────────────────────

    /// Refresh the camera list from `scrcpy -s SERIAL --list-cameras`.
    pub fn refresh_cameras(&mut self) {
        let Some(serial) = self.selected_info().map(|d| d.serial.clone()) else {
            self.log_err("No hay dispositivo seleccionado");
            return;
        };

        self.log("Obteniendo cámaras...");
        let output = match std::process::Command::new("scrcpy")
            .args(["-s", &serial, "--list-cameras"])
            .output()
        {
            Ok(o) => o,
            Err(e) => {
                self.log_err(format!("Error ejecutando scrcpy --list-cameras: {e}"));
                return;
            }
        };

        let text = String::from_utf8_lossy(&output.stdout);
        let cameras = parse_camera_list(&text);

        if cameras.is_empty() {
            self.log_err("No se encontraron cámaras");
            return;
        }

        self.camera.available = cameras;
        self.camera.selected_camera = 0;
        self.camera.selected_size = 0;
        self.log_ok(format!("{} cámaras encontradas", self.camera.available.len()));
    }

    /// Select a camera by index in the available list.
    pub fn select_camera(&mut self, idx: usize) {
        if idx < self.camera.available.len() {
            self.camera.selected_camera = idx;
            self.camera.selected_size = 0;
        }
    }

    /// Select a camera size preset by index.
    pub fn select_camera_size(&mut self, idx: usize) {
        let presets = self.camera.size_presets();
        if idx < presets.len() {
            self.camera.selected_size = idx;
            self.log(format!("Tamaño cámara: {}", presets[idx]));
        }
    }

    /// Cycle between h264 and h265.
    pub fn cycle_camera_codec(&mut self) {
        if self.camera.video_codec == "h264" {
            self.camera.video_codec = "h265".to_string();
        } else {
            self.camera.video_codec = "h264".to_string();
        }
        self.log(format!("Codec: {}", self.camera.video_codec));
    }

    /// Toggle the no-window flag.
    pub fn toggle_camera_no_window(&mut self) {
        self.camera.no_window = !self.camera.no_window;
        self.log(if self.camera.no_window {
            "Sin ventana: ON".to_string()
        } else {
            "Sin ventana: OFF".to_string()
        });
    }

    /// Toggle logs modal.
    pub fn toggle_logs(&mut self) {
        self.show_logs = !self.show_logs;
    }

    /// Build and return the preview of the scrcpy camera command.
    pub fn camera_command_preview(&self) -> String {
        let Some(serial) = self.selected_info().map(|d| d.serial.as_str()) else {
            return String::new();
        };

        let mut lines = vec![
            format!("scrcpy -s {serial}"),
            "    --video-source=camera".to_string(),
        ];
        if let Some(cam) = self.camera.selected_camera_info() {
            lines.push(format!("    --camera-id={}", cam.id));
        }
        lines.push(format!("    --camera-size={}", self.camera.current_size()));
        lines.push(format!("    --camera-zoom={}", self.camera.camera_zoom));
        lines.push(format!("    --camera-fps={}", self.camera.camera_fps));
        lines.push(format!("    --video-codec={}", self.camera.video_codec));
        lines.push(format!("    --v4l2-sink={}", self.camera.v4l2_sink));
        if self.camera.no_window {
            lines.push("    --no-window".to_string());
        }
        lines.join("\n")
    }

    /// Launch scrcpy with camera source settings.
    pub fn launch_camera_scrcpy(&mut self) {
        let Some(serial) = self.selected_info().map(|d| d.serial.clone()) else {
            self.log_err("No hay dispositivo seleccionado");
            return;
        };

        self.log("Lanzando scrcpy-cámara...");
        let mut cmd = std::process::Command::new("scrcpy");
        cmd.args(["-s", &serial, "--video-source=camera"]);
        if let Some(cam) = self.camera.selected_camera_info() {
            cmd.args(["--camera-id", &cam.id]);
        }
        cmd.args([
            "--camera-size",
            &self.camera.current_size(),
            "--camera-zoom",
            &self.camera.camera_zoom,
            "--camera-fps",
            &self.camera.camera_fps,
            "--video-codec",
            &self.camera.video_codec,
            "--v4l2-sink",
            &self.camera.v4l2_sink,
        ]);
        if self.camera.no_window {
            cmd.arg("--no-window");
        }
        cmd.stdout(std::process::Stdio::null());
        cmd.stderr(std::process::Stdio::piped());

        match cmd.spawn() {
            Ok(mut child) => {
                let Some(stderr) = child.stderr.take() else {
                    self.log_err("No se pudo capturar stderr de scrcpy");
                    return;
                };
                let (tx, rx) = mpsc::channel();
                self.scrcpy_rx = Some(rx);

                std::thread::spawn(move || {
                    use std::io::BufRead;
                    let reader = std::io::BufReader::new(stderr);
                    for line in reader.lines() {
                        match line {
                            Ok(msg) => {
                                if tx.send(msg).is_err() {
                                    break;
                                }
                            }
                            Err(_) => break,
                        }
                    }
                    let _ = child.wait();
                });

                self.log_ok("scrcpy-cámara lanzado");
            }
            Err(e) => self.log_err(format!("Error lanzando scrcpy-cámara: {e}")),
        }
    }
}

/// Check whether `scrcpy` is available in PATH.
pub fn check_scrcpy() -> bool {
    std::process::Command::new("scrcpy")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

/// Parse the output of `scrcpy --list-cameras`.
///
/// Expected line format:
///     --camera-id=0    (back, 4000x3000, fps={10, 15, 20}, zoom-range=[1, 10])
pub fn parse_camera_list(output: &str) -> Vec<CameraInfo> {
    output
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            let rest = line.strip_prefix("--camera-id=")?;
            let (id, rest) = rest.split_once(|c: char| c.is_whitespace())?;
            let inner = rest.trim().strip_prefix('(')?;
            let (name, rest) = inner.split_once(", ")?;
            let resolution = rest.split(", ").next()?;
            Some(CameraInfo {
                id: id.to_string(),
                name: name.to_string(),
                max_resolution: resolution.to_string(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adb::mock::MockAdbClient;

    fn make_app(devices: Vec<String>) -> App {
        let mut mock = MockAdbClient::new();
        mock.devices = devices;
        App::new(Box::new(mock))
    }

    #[test]
    fn test_no_devices() {
        let app = make_app(vec![]);
        assert!(app.devices.is_empty());
        assert!(!app.has_selected());
    }

    #[test]
    fn test_single_device() {
        let app = make_app(vec!["R58MA123".to_string()]);
        assert_eq!(app.devices.len(), 1);
        assert_eq!(app.devices[0].serial, "R58MA123");
        assert_eq!(app.devices[0].connection, ConnectionType::Usb);
        assert!(app.is_selected_usb());
        assert!(!app.is_selected_wifi());
    }

    #[test]
    fn test_tcpip_device() {
        let mut app = make_app(vec!["192.168.1.100:5555".to_string()]);
        app.select_device(0);
        assert_eq!(app.devices.len(), 1);
        assert_eq!(app.devices[0].connection, ConnectionType::TcpIp);
        assert!(app.is_selected_wifi());
        assert!(!app.is_selected_usb());
        assert_eq!(app.manual_ip, "192.168.1.100");
    }

    #[test]
    fn test_log_basic() {
        let mut app = make_app(vec![]);
        let initial = app.logs.len();
        app.log("test");
        assert_eq!(app.logs.len(), initial + 1);
        assert!(app.logs.last().unwrap().contains("test"));
    }

    #[test]
    fn test_log_ok_err() {
        let mut app = make_app(vec![]);
        let initial = app.logs.len();
        app.log_ok("exitoso");
        app.log_err("fallo");
        assert!(app.logs[initial].starts_with("\u{f00c}"));
        assert!(app.logs[initial + 1].starts_with("\u{f00d}"));
    }

    #[test]
    fn test_log_capacity() {
        let mut app = make_app(vec![]);
        for i in 0..150 {
            app.log(format!("msg {i}"));
        }
        assert!(app.logs.len() <= 100);
        assert!(app.logs[0].contains("msg 50"));
    }

    #[test]
    fn test_select_device() {
        let mut app = make_app(vec![
            "dev1".to_string(),
            "dev2".to_string(),
            "192.168.1.1:5555".to_string(),
        ]);
        assert_eq!(app.selected, 0);
        app.select_device(2);
        assert_eq!(app.selected, 2);
        assert!(app.is_selected_wifi());
        assert_eq!(app.manual_ip, "192.168.1.1");
    }

    #[test]
    fn test_select_device_out_of_bounds() {
        let mut app = make_app(vec!["dev1".to_string()]);
        app.select_device(5);
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn test_cycle_bitrate() {
        let mut app = make_app(vec![]);
        assert_eq!(app.quality.bitrate, "");

        app.cycle_bitrate();
        assert_eq!(app.quality.bitrate, "4M");

        app.cycle_bitrate();
        assert_eq!(app.quality.bitrate, "8M");

        app.cycle_bitrate();
        assert_eq!(app.quality.bitrate, "16M");

        app.cycle_bitrate();
        assert_eq!(app.quality.bitrate, "4M");
    }

    #[test]
    fn test_cycle_fps() {
        let mut app = make_app(vec![]);
        assert_eq!(app.quality.max_fps, 0);

        app.cycle_fps();
        assert_eq!(app.quality.max_fps, 30);

        app.cycle_fps();
        assert_eq!(app.quality.max_fps, 60);

        app.cycle_fps();
        assert_eq!(app.quality.max_fps, 30);
    }

    #[test]
    fn test_cycle_maxsize() {
        let mut app = make_app(vec![]);
        assert_eq!(app.quality.max_size, 0);

        app.cycle_maxsize();
        assert_eq!(app.quality.max_size, 720);

        app.cycle_maxsize();
        assert_eq!(app.quality.max_size, 1080);

        app.cycle_maxsize();
        assert_eq!(app.quality.max_size, 1920);

        app.cycle_maxsize();
        assert_eq!(app.quality.max_size, 0);
    }

    #[test]
    fn test_disconnect_no_device() {
        let mut app = make_app(vec![]);
        app.disconnect_selected();
        assert!(app.logs.last().unwrap().contains("No hay dispositivo"));
    }

    #[test]
    fn test_make_wireless_no_device() {
        let mut app = make_app(vec![]);
        app.make_wireless();
        assert_eq!(app.wireless_state, WirelessState::Idle);
        assert!(app.logs.last().unwrap().contains("No hay dispositivo"));
    }

    #[test]
    fn test_make_wireless_already_wifi() {
        let mut app = make_app(vec!["192.168.1.1:5555".to_string()]);
        app.make_wireless();
        assert!(app.logs.last().unwrap().contains("ya está en modo WiFi"));
    }

    #[test]
    fn test_make_wireless_starts_process() {
        let mut mock = MockAdbClient::new();
        mock.devices = vec!["R58MA123".to_string()];
        mock.set_tcpip_ok = true;
        let mut app = App::new(Box::new(mock));

        app.make_wireless();
        assert_eq!(
            app.wireless_state,
            WirelessState::WaitingTcpIp {
                serial: "R58MA123".to_string(),
                attempts: 0,
                launch_scrcpy: false,
            }
        );
    }

    #[test]
    fn test_tick_wireless_finds_ip_and_connects() {
        let mut mock = MockAdbClient::new();
        mock.devices = vec!["R58MA123".to_string()];
        mock.phone_ips = vec![Some("192.168.1.100".to_string())];
        mock.connect_ok = true;
        let mut app = App::new(Box::new(mock));

        app.wireless_state = WirelessState::WaitingTcpIp {
            serial: "R58MA123".to_string(),
            attempts: 5,
            launch_scrcpy: false,
        };
        app.tick_wireless();

        assert_eq!(app.wireless_state, WirelessState::Idle);
        assert!(app.logs.iter().any(|l| l.contains("Conectado por WiFi")));
    }

    #[test]
    fn test_tick_wireless_exhausts_attempts() {
        let mut mock = MockAdbClient::new();
        mock.devices = vec!["R58MA123".to_string()];
        mock.phone_ips = vec![None];
        let mut app = App::new(Box::new(mock));

        app.wireless_state = WirelessState::WaitingTcpIp {
            serial: "R58MA123".to_string(),
            attempts: 30,
            launch_scrcpy: false,
        };
        app.tick_wireless();

        assert_eq!(app.wireless_state, WirelessState::Idle);
        assert!(app.logs.iter().any(|l| l.contains("No se pudo obtener IP")));
    }

    #[test]
    fn test_connect_selected_wifi_launches_scrcpy() {
        let mut mock = MockAdbClient::new();
        mock.devices = vec!["192.168.1.1:5555".to_string()];
        let mut app = App::new(Box::new(mock));

        app.connect_selected();
        assert!(app.logs.iter().any(|l| {
            l.contains("scrcpy lanzado")
                || l.contains("scrcpy")
        }));
    }

    #[test]
    fn test_is_tcpip() {
        assert!(adb::is_tcpip("192.168.1.1:5555"));
        assert!(!adb::is_tcpip("R58MA123ABC"));
    }

    #[test]
    fn test_panel_default_main() {
        let app = make_app(vec![]);
        assert_eq!(app.panel, Panel::Main);
    }

    #[test]
    fn test_show_logs_default_false() {
        let app = make_app(vec![]);
        assert!(!app.show_logs);
    }

    #[test]
    fn test_toggle_logs() {
        let mut app = make_app(vec![]);
        app.toggle_logs();
        assert!(app.show_logs);
        app.toggle_logs();
        assert!(!app.show_logs);
    }

    #[test]
    fn test_camera_defaults() {
        let app = make_app(vec![]);
        assert!(app.camera.available.is_empty());
        assert_eq!(app.camera.camera_zoom, "1");
        assert_eq!(app.camera.camera_fps, "30");
        assert_eq!(app.camera.video_codec, "h264");
        assert_eq!(app.camera.v4l2_sink, "/dev/video0");
        assert!(!app.camera.no_window);
    }

    #[test]
    fn test_cycle_camera_codec() {
        let mut app = make_app(vec![]);
        assert_eq!(app.camera.video_codec, "h264");
        app.cycle_camera_codec();
        assert_eq!(app.camera.video_codec, "h265");
        app.cycle_camera_codec();
        assert_eq!(app.camera.video_codec, "h264");
    }

    #[test]
    fn test_toggle_camera_no_window() {
        let mut app = make_app(vec![]);
        assert!(!app.camera.no_window);
        app.toggle_camera_no_window();
        assert!(app.camera.no_window);
        app.toggle_camera_no_window();
        assert!(!app.camera.no_window);
    }

    #[test]
    fn test_camera_size_presets_without_camera() {
        let app = make_app(vec![]);
        let presets = app.camera.size_presets();
        assert_eq!(presets, vec!["640x480", "1280x720", "1920x1080"]);
    }

    #[test]
    fn test_parse_camera_list() {
        let output = "\
scrcpy 4.0
INFO: ADB device found:
    --camera-id=0    (back, 4000x3000, fps={10, 15, 20, 24, 30}, zoom-range=[1, 10])
    --camera-id=1    (front, 2592x1944, fps={10, 15, 20, 30}, zoom-range=[1, 10])
";
        let cameras = parse_camera_list(output);
        assert_eq!(cameras.len(), 2);
        assert_eq!(cameras[0].id, "0");
        assert_eq!(cameras[0].name, "back");
        assert_eq!(cameras[0].max_resolution, "4000x3000");
        assert_eq!(cameras[1].id, "1");
        assert_eq!(cameras[1].name, "front");
        assert_eq!(cameras[1].max_resolution, "2592x1944");
    }

    #[test]
    fn test_parse_camera_list_empty() {
        assert!(parse_camera_list("no cameras here").is_empty());
    }

    #[test]
    fn test_select_camera_bounds() {
        let mut app = make_app(vec![]);
        app.camera.available = vec![
            CameraInfo { id: "0".into(), name: "back".into(), max_resolution: "4000x3000".into() },
            CameraInfo { id: "1".into(), name: "front".into(), max_resolution: "2592x1944".into() },
        ];
        app.select_camera(0);
        assert_eq!(app.camera.selected_camera, 0);
        app.select_camera(1);
        assert_eq!(app.camera.selected_camera, 1);
        app.select_camera(5);
        assert_eq!(app.camera.selected_camera, 1);
    }

    #[test]
    fn test_camera_size_presets_with_selected() {
        let mut app = make_app(vec![]);
        app.camera.available = vec![
            CameraInfo { id: "0".into(), name: "back".into(), max_resolution: "4000x3000".into() },
        ];
        app.select_camera(0);
        let presets = app.camera.size_presets();
        assert_eq!(presets, vec!["640x480", "1280x720", "1920x1080", "4000x3000"]);
    }

    #[test]
    fn test_camera_command_preview_no_device() {
        let app = make_app(vec![]);
        assert!(app.camera_command_preview().is_empty());
    }


}
