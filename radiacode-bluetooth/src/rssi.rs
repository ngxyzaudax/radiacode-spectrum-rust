#[cfg(target_os = "linux")]
#[path = "rssi_linux.rs"]
mod rssi_linux;

#[cfg(target_os = "linux")]
#[path = "rssi_mgmt.rs"]
pub mod rssi_mgmt;

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

pub async fn read_mgmt_rssi_dbm(mac: &str) -> Option<i16> {
    #[cfg(target_os = "linux")]
    {
        return rssi_mgmt::read_connected_rssi_dbm(mac).await;
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = mac;
        None
    }
}

pub fn read_dbus_rssi_dbm(mac: &str) -> Option<i16> {
    #[cfg(target_os = "linux")]
    {
        return rssi_linux::read_dbus_rssi_dbm(mac);
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = mac;
        None
    }
}
