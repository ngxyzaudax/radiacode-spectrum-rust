use tracing::debug;

#[path = "rssi_mgmt.rs"]
mod rssi_mgmt;

pub async fn read_connected_rssi_dbm(mac: &str) -> Option<i16> {
    if let Some(rssi) = rssi_mgmt::read_connected_rssi_dbm(mac).await {
        return Some(rssi);
    }
    read_dbus_rssi_dbm(mac)
}

pub fn read_dbus_rssi_dbm(mac: &str) -> Option<i16> {
    let path = bluez_device_path(mac);
    let output = std::process::Command::new("dbus-send")
        .args([
            "--system",
            "--print-reply",
            "--dest=org.bluez",
            &path,
            "org.freedesktop.DBus.Properties.Get",
            "string:org.bluez.Device1",
            "string:RSSI",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let rssi = parse_dbus_variant_int16(&output.stdout)?;
    debug!(%mac, rssi, "rssi from bluez property fallback");
    Some(rssi)
}

fn bluez_device_path(mac: &str) -> String {
    format!(
        "/org/bluez/hci0/dev_{}",
        mac.to_uppercase().replace(':', "_")
    )
}

fn parse_dbus_variant_int16(stdout: &[u8]) -> Option<i16> {
    let text = String::from_utf8_lossy(stdout);
    let mut saw_variant = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("variant") {
            saw_variant = true;
            continue;
        }
        if saw_variant && trimmed.starts_with("int16 ") {
            return trimmed.strip_prefix("int16 ")?.parse().ok();
        }
    }
    None
}
