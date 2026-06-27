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

/// Display language for the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Es,
}

impl Lang {
    pub fn toggle(&mut self) {
        *self = match self {
            Lang::En => Lang::Es,
            Lang::Es => Lang::En,
        };
    }

    pub fn label(self) -> &'static str {
        match self {
            Lang::En => " EN ",
            Lang::Es => " ES ",
        }
    }
}

/// Return the string for the current language.
pub fn tr(lang: Lang, es: &'static str, en: &'static str) -> &'static str {
    match lang {
        Lang::Es => es,
        Lang::En => en,
    }
}


