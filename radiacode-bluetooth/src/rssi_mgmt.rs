use std::time::Duration;

use bdaddr::Address;
use btmgmt::Client;
use btmgmt::command::GetConnectionInformation;
use futures::StreamExt;
use tokio::time::timeout;
use tracing::debug;

const MGMT_RSSI_TIMEOUT: Duration = Duration::from_millis(400);

pub async fn read_connected_rssi_dbm(mac: &str) -> Option<i16> {
    let probe = async {
        if let Some(rssi) = read_for_address(mac, Address::le_public_from_str(mac).ok()).await {
            return Some(rssi);
        }
        read_for_address(mac, Address::le_random_from_str(mac).ok()).await
    };
    timeout(MGMT_RSSI_TIMEOUT, probe).await.ok().flatten()
}

async fn read_for_address(mac: &str, address: Option<Address>) -> Option<i16> {
    let address = address?;
    let client = Client::open().ok()?;
    let mut events = client.events().await;
    tokio::spawn(async move {
        while events.next().await.is_some() {}
    });
    let reply = client
        .call(None, GetConnectionInformation::new(address))
        .await
        .ok()?;
    let rssi = *reply.rssi() as i8;
    if rssi == 127 {
        return None;
    }
    debug!(%mac, rssi, "rssi from mgmt conn-info");
    Some(i16::from(rssi))
}
