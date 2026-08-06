#[cfg(target_os = "linux")]
#[path = "rssi_linux.rs"]
mod rssi_linux;

pub async fn read_connected_rssi_dbm(mac: &str) -> Option<i16> {
    #[cfg(target_os = "linux")]
    {
        return rssi_linux::read_connected_rssi_dbm(mac).await;
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = mac;
        None
    }
}
