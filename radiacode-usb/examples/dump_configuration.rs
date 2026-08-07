use radiacode_core::VirtString;
use radiacode_usb::{connect, scan_usb_devices};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let devices = scan_usb_devices()?;
    let mut device = connect(&devices[0].endpoint.address_label()).await?;
    let config = device.read_virt_string(VirtString::Configuration).await?;
    let text = String::from_utf8_lossy(config.data());
    for line in text.lines() {
        println!("{line}");
    }
    let _ = device.disconnect().await;
    Ok(())
}
