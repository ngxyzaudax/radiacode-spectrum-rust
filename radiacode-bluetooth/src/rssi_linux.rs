use tracing::debug;

pub async fn read_connected_rssi_dbm(mac: &str) -> Option<i16> {
    let path = bluez_device_path(mac);
    if let Some(rssi) = dbus_read_rssi_method(&path).await {
        debug!(%mac, rssi, "rssi from bluez ReadRSSI");
        return Some(rssi);
    }
    if let Some(rssi) = dbus_read_rssi_property(&path).await {
        debug!(%mac, rssi, "rssi from bluez property");
        return Some(rssi);
    }
    None
}

fn bluez_device_path(mac: &str) -> String {
    format!(
        "/org/bluez/hci0/dev_{}",
        mac.to_uppercase().replace(':', "_")
    )
}

async fn dbus_read_rssi_method(path: &str) -> Option<i16> {
    let output = tokio::process::Command::new("dbus-send")
        .args([
            "--system",
            "--print-reply",
            "--dest=org.bluez",
            path,
            "org.bluez.Device1.ReadRSSI",
        ])
        .output()
        .await
        .ok()?;
    if !output.status.success() {
        return None;
    }
    parse_dbus_int16_reply(&output.stdout)
}

async fn dbus_read_rssi_property(path: &str) -> Option<i16> {
    let output = tokio::process::Command::new("dbus-send")
        .args([
            "--system",
            "--print-reply",
            "--dest=org.bluez",
            path,
            "org.freedesktop.DBus.Properties.Get",
            "string:org.bluez.Device1",
            "string:RSSI",
        ])
        .output()
        .await
        .ok()?;
    if !output.status.success() {
        return None;
    }
    parse_dbus_variant_int16(&output.stdout)
}

fn parse_dbus_int16_reply(stdout: &[u8]) -> Option<i16> {
    let text = String::from_utf8_lossy(stdout);
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("int16 ") {
            return trimmed.strip_prefix("int16 ")?.parse().ok();
        }
    }
    None
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
