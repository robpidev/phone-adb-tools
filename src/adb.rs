use std::process::Command;

pub fn check_adb() -> bool {
    Command::new("adb").arg("--version").output().is_ok()
}

pub fn list_devices() -> Vec<String> {
    let output = Command::new("adb").arg("devices").output();
    let Ok(output) = output else {
        return vec![];
    };
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines()
        .skip(1)
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let serial = parts.next()?;
            let state = parts.next()?;
            if state == "device" {
                Some(serial.to_string())
            } else {
                None
            }
        })
        .collect()
}

pub fn is_tcpip(serial: &str) -> bool {
    serial.contains(':')
}

pub fn fetch_device_props(serial: &str) -> (String, String) {
    let model = Command::new("adb")
        .args(["-s", serial, "shell", "getprop", "ro.product.model"])
        .output()
        .ok()
        .and_then(|o| {
            let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if s.is_empty() {
                None
            } else {
                Some(s)
            }
        })
        .unwrap_or_default();

    let version = Command::new("adb")
        .args(["-s", serial, "shell", "getprop", "ro.build.version.release"])
        .output()
        .ok()
        .and_then(|o| {
            let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if s.is_empty() {
                None
            } else {
                Some(s)
            }
        })
        .unwrap_or_default();

    (model, version)
}

pub fn get_phone_ip(serial: &str) -> Option<String> {
    for iface in &["wlan0", "eth0", "usb0", "wlan1"] {
        if let Some(ip) = get_ip_for_iface(serial, iface) {
            return Some(ip);
        }
    }
    None
}

fn get_ip_for_iface(serial: &str, iface: &str) -> Option<String> {
    let output = Command::new("adb")
        .args(["-s", serial, "shell", "ip", "addr", "show", iface])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with("inet ") {
            let ip = line.split_whitespace().nth(1)?;
            return Some(ip.split('/').next()?.to_string());
        }
    }
    None
}

pub fn set_tcpip(serial: &str, port: u16) -> bool {
    Command::new("adb")
        .args(["-s", serial, "tcpip", &port.to_string()])
        .status()
        .is_ok()
}

pub fn connect_to(target: &str) -> bool {
    Command::new("adb")
        .args(["connect", target])
        .status()
        .is_ok()
}

pub fn disconnect_from(serial: &str) -> bool {
    Command::new("adb")
        .args(["disconnect", serial])
        .status()
        .is_ok()
}
