use std::os::unix::process::CommandExt;
use std::sync::mpsc;

use crate::types::CameraInfo;

use super::App;

impl App {
    /// Refresh the camera list from `scrcpy -s SERIAL --list-cameras`.
    pub fn refresh_cameras(&mut self) {
        self.loading = true;
        let Some(serial) = self.selected_info().map(|d| d.serial.clone()) else {
            self.log_err(self.tr("No hay dispositivo seleccionado", "No device selected"));
            self.loading = false;
            return;
        };

        self.log(self.tr("Obteniendo cámaras...", "Getting cameras..."));
        let output = match std::process::Command::new("scrcpy")
            .args(["-s", &serial, "--list-cameras"])
            .output()
        {
            Ok(o) => o,
            Err(e) => {
                self.log_err(format!(
                    "{}: {e}",
                    self.tr("Error ejecutando scrcpy --list-cameras", "Error running scrcpy --list-cameras")
                ));
                self.loading = false;
                return;
            }
        };

        let text = String::from_utf8_lossy(&output.stdout);
        let cameras = parse_camera_list(&text);

        if cameras.is_empty() {
            self.log_err(self.tr("No se encontraron cámaras", "No cameras found"));
            self.loading = false;
            return;
        }

        self.camera.available = cameras;
        self.camera.selected_camera = 0;
        self.camera.selected_size = 0;
        self.log_ok(format!(
            "{} {}",
            self.camera.available.len(),
            self.tr("cámaras encontradas", "cameras found")
        ));
        self.loading = false;
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
            self.log(format!(
                "{}: {}",
                self.tr("Tamaño cámara", "Camera size"),
                presets[idx]
            ));
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
            format!("{}: ON", self.tr("Sin ventana", "No window"))
        } else {
            format!("{}: OFF", self.tr("Sin ventana", "No window"))
        });
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
            self.log_err(self.tr("No hay dispositivo seleccionado", "No device selected"));
            return;
        };

        self.log(self.tr("Lanzando scrcpy-cámara...", "Launching scrcpy-camera..."));
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
        cmd.process_group(0);

        match cmd.spawn() {
            Ok(mut child) => {
                let Some(stderr) = child.stderr.take() else {
                    self.log_err(self.tr("No se pudo capturar stderr de scrcpy", "Could not capture scrcpy stderr"));
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

                self.log_ok(self.tr("scrcpy-cámara lanzado", "scrcpy-camera launched"));
            }
            Err(e) => self.log_err(format!(
                "{}: {e}",
                self.tr("Error lanzando scrcpy-cámara", "Error launching scrcpy-camera")
            )),
        }
    }
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
