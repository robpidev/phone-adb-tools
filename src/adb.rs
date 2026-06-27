use std::process::{Command, Output};
use anyhow::{Context, Result};

/// ADB operations trait, allowing mocking in tests.
pub trait AdbClient {
    /// Check that `adb` is available in PATH.
    fn check_adb(&self) -> Result<()>;
    /// List device serials that are in "device" state.
    fn list_devices(&self) -> Result<Vec<String>>;
    /// Fetch model and Android version for a device.
    fn fetch_device_props(&self, serial: &str) -> Result<(String, String)>;
    /// Attempt to discover the device's IP address.
    fn get_phone_ip(&self, serial: &str) -> Result<Option<String>>;
    /// Switch a USB device to TCP/IP mode on the given port.
    fn set_tcpip(&self, serial: &str, port: u16) -> Result<()>;
    /// Connect to a device at `target` (IP:port).
    fn connect_to(&self, target: &str) -> Result<()>;
    /// Disconnect a wireless device.
    fn disconnect_from(&self, serial: &str) -> Result<()>;
}

/// Real [`AdbClient`] that shells out to the system `adb` command.
pub struct RealAdbClient;

impl AdbClient for RealAdbClient {
    fn check_adb(&self) -> Result<()> {
        run("adb", &["--version"])
    }

    fn list_devices(&self) -> Result<Vec<String>> {
        let output = output("adb", &["devices"])?;
        let text = String::from_utf8_lossy(&output.stdout);
        Ok(text
            .lines()
            .skip(1)
            .filter_map(|line| {
                let mut parts = line.split_whitespace();
                let serial = parts.next()?;
                let state = parts.next()?;
                (state == "device").then(|| serial.to_string())
            })
            .collect())
    }

    fn fetch_device_props(&self, serial: &str) -> Result<(String, String)> {
        let model = get_prop(serial, "ro.product.model")?;
        let version = get_prop(serial, "ro.build.version.release")?;
        Ok((model, version))
    }

    fn get_phone_ip(&self, serial: &str) -> Result<Option<String>> {
        for iface in &["wlan0", "eth0", "usb0", "wlan1"] {
            if let Ok(Some(ip)) = get_ip_for_iface(serial, iface) {
                return Ok(Some(ip));
            }
        }
        Ok(None)
    }

    fn set_tcpip(&self, serial: &str, port: u16) -> Result<()> {
        run("adb", &["-s", serial, "tcpip", &port.to_string()])
    }

    fn connect_to(&self, target: &str) -> Result<()> {
        run("adb", &["connect", target])
    }

    fn disconnect_from(&self, serial: &str) -> Result<()> {
        run("adb", &["disconnect", serial])
    }
}

fn run(cmd: &str, args: &[&str]) -> Result<()> {
    Command::new(cmd)
        .args(args)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .context(format!("Error executing {cmd}"))?
        .success()
        .then_some(())
        .context(format!("{cmd} finished with error"))
}

fn output(cmd: &str, args: &[&str]) -> Result<Output> {
    Command::new(cmd)
        .args(args)
        .output()
        .context(format!("Error running {cmd}"))
}

fn get_prop(serial: &str, prop: &str) -> Result<String> {
    let output = output("adb", &["-s", serial, "shell", "getprop", prop])?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn get_ip_for_iface(serial: &str, iface: &str) -> Result<Option<String>> {
    let output = output("adb", &["-s", serial, "shell", "ip", "addr", "show", iface])?;
    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with("inet ") {
            let ip = line
                .split_whitespace()
                .nth(1)
                .context("unexpected format in ip addr show")?;
            return Ok(Some(
                ip.split('/')
                    .next()
                    .context("unexpected format in ip addr show")?
                    .to_string(),
            ));
        }
    }
    Ok(None)
}

/// Check whether a serial string identifies a TCP/IP device (contains `:`).
pub fn is_tcpip(serial: &str) -> bool {
    serial.contains(':')
}

#[cfg(test)]
pub(crate) mod mock {
    use super::*;

    pub struct MockAdbClient {
        pub check_adb_result: Result<()>,
        pub devices: Vec<String>,
        pub props: Vec<(String, String)>,
        pub phone_ips: Vec<Option<String>>,
        pub set_tcpip_ok: bool,
        pub connect_ok: bool,
        pub disconnect_ok: bool,
    }

    impl MockAdbClient {
        #[allow(clippy::new_without_default)]
        pub fn new() -> Self {
            Self {
                check_adb_result: Ok(()),
                devices: Vec::new(),
                props: Vec::new(),
                phone_ips: Vec::new(),
                set_tcpip_ok: true,
                connect_ok: true,
                disconnect_ok: true,
            }
        }
    }

    impl AdbClient for MockAdbClient {
        fn check_adb(&self) -> Result<()> {
            match &self.check_adb_result {
                Ok(()) => Ok(()),
                Err(e) => Err(anyhow::anyhow!("{}", e)),
            }
        }

        fn list_devices(&self) -> Result<Vec<String>> {
            Ok(self.devices.clone())
        }

        fn fetch_device_props(&self, _serial: &str) -> Result<(String, String)> {
            Ok(self.props.first().cloned().unwrap_or_default())
        }

        fn get_phone_ip(&self, _serial: &str) -> Result<Option<String>> {
            Ok(self.phone_ips.first().cloned().unwrap_or(None))
        }

        fn set_tcpip(&self, _serial: &str, _port: u16) -> Result<()> {
            if self.set_tcpip_ok {
                Ok(())
            } else {
                Err(anyhow::anyhow!("set_tcpip failed"))
            }
        }

        fn connect_to(&self, _target: &str) -> Result<()> {
            if self.connect_ok {
                Ok(())
            } else {
                Err(anyhow::anyhow!("connect failed"))
            }
        }

        fn disconnect_from(&self, _serial: &str) -> Result<()> {
            if self.disconnect_ok {
                Ok(())
            } else {
                Err(anyhow::anyhow!("disconnect failed"))
            }
        }
    }
}
