use radiacode_core::VirtString;
use radiacode_usb::{connect, scan_usb_devices};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let devices = scan_usb_devices()?;
    let mut device = connect(&devices[0].endpoint.address_label()).await?;
    let sfr = device.read_virt_string(VirtString::SfrFile).await?;
    print!("{}", String::from_utf8_lossy(sfr.data()));
    let _ = device.disconnect().await;
    Ok(())
}
