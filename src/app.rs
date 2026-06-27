use std::{
    sync::mpsc,
    time::{Duration, Instant},
};

use crate::adb;

#[derive(Clone, Copy, PartialEq)]
pub enum ConnectionType {
    Usb,
    TcpIp,
}

#[derive(Clone)]
pub struct DeviceInfo {
    pub serial: String,
    pub model: String,
    pub android_version: String,
    pub connection: ConnectionType,
}

#[derive(Clone, Copy, PartialEq)]
pub enum EditMode {
    None,
    Port,
    Bitrate,
    Fps,
    MaxSize,
    ManualIp,
}

#[derive(Clone, Default)]
pub struct ScrcpyQuality {
    pub bitrate: String,
    pub max_size: u16,
    pub max_fps: u16,
}

pub struct App {
    pub devices: Vec<DeviceInfo>,
    pub selected: usize,
    pub logs: Vec<String>,
    pub last_refresh: Option<Instant>,
    pub port: u16,
    pub manual_ip: String,
    pub edit_mode: EditMode,
    pub edit_buffer: String,
    pub center_focus: usize,
    pub quality: ScrcpyQuality,
    pub show_help: bool,
    pub scrcpy_rx: Option<mpsc::Receiver<String>>,
}

impl App {
    pub fn log<S: Into<String>>(&mut self, msg: S) {
        self.logs.push(msg.into());
        if self.logs.len() > 100 {
            self.logs.remove(0);
        }
    }

    pub fn refresh_devices(&mut self) {
        self.log("\u{f021} Refrescando dispositivos...");
        let serials = adb::list_devices();
        let mut devices = Vec::with_capacity(serials.len());

        for serial in &serials {
            let connection = if adb::is_tcpip(serial) {
                ConnectionType::TcpIp
            } else {
                ConnectionType::Usb
            };
            let (model, version) = adb::fetch_device_props(serial);
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
        self.log(format!("Encontrados {} dispositivos", self.devices.len()));
    }

    pub fn selected_device(&self) -> Option<String> {
        self.devices.get(self.selected).map(|d| d.serial.clone())
    }

    pub fn selected_info(&self) -> Option<&DeviceInfo> {
        self.devices.get(self.selected)
    }

    pub fn connect_selected(&mut self) {
        self.make_wireless();
        self.launch_scrcpy();
    }

    pub fn make_wireless(&mut self) {
        let Some(serial) = self.selected_device() else {
            self.log("No hay dispositivo seleccionado");
            return;
        };

        if adb::is_tcpip(&serial) {
            self.log("El dispositivo ya está en modo WiFi");
            return;
        }

        self.log(format!("Activando TCP/IP en {serial}"));

        if !adb::set_tcpip(&serial, self.port) {
            self.log("\u{f00d} Error activando TCP/IP");
            return;
        }
        self.log("\u{f00c} TCP/IP activado");

        std::thread::sleep(Duration::from_secs(2));

        let Some(ip) = adb::get_phone_ip(&serial) else {
            self.log("\u{f00d} No se pudo obtener IP");
            return;
        };

        self.log(format!("\u{f00c} IP detectada: {ip}"));

        let target = format!("{}:{}", ip, self.port);
        if !adb::connect_to(&target) {
            self.log("\u{f00d} Error conectando por WiFi");
            return;
        }
        self.log("\u{f00c} Conectado por WiFi");
        self.refresh_devices();
    }

    pub fn launch_scrcpy(&mut self) {
        let Some(serial) = self.selected_device() else {
            self.log("No hay dispositivo seleccionado");
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
        cmd.stderr(std::process::Stdio::piped());

        match cmd.spawn() {
            Ok(mut child) => {
                let stderr = child.stderr.take().expect("stderr piped");
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

                self.log("\u{f00c} scrcpy lanzado");
            }
            Err(e) => self.log(format!("\u{f00d} Error lanzando scrcpy: {e}")),
        }
    }

    pub fn connect_manual(&mut self) {
        let target = self.manual_ip.trim().to_string();
        if target.is_empty() {
            self.log("No se ingresó dirección");
            return;
        }

        let full_target = if target.contains(':') {
            target
        } else {
            format!("{}:{}", target, self.port)
        };

        self.log(format!("Conectando a {full_target}..."));
        if adb::connect_to(&full_target) {
            self.log("\u{f00c} Conectado");
            self.refresh_devices();
        } else {
            self.log("\u{f00d} Error conectando");
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
        let pos = PRESETS.iter().position(|&p| p == self.quality.max_size).unwrap_or(3);
        let next = (pos + 1) % PRESETS.len();
        self.quality.max_size = PRESETS[next];
        self.log(format!(
            "Max size: {}",
            if self.quality.max_size == 0 { "sin límite".to_string() } else { self.quality.max_size.to_string() }
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
        let Some(serial) = self.selected_device() else {
            self.log("No hay dispositivo seleccionado");
            return;
        };

        self.log(format!("Desconectando {serial}..."));
        if adb::disconnect_from(&serial) {
            self.log("\u{f00c} Desconectado");
            self.refresh_devices();
        } else {
            self.log(format!("\u{f00d} Error desconectando {serial}"));
        }
    }
}
