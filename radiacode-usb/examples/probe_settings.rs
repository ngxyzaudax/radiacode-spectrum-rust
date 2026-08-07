use radiacode_usb::{connect, scan_usb_devices};

#[tokio::main]
async fn main() {
    let devices = scan_usb_devices().expect("usb scan");
    let serial = devices[0].endpoint.address_label().to_string();
    let mut device = connect(&serial).await.expect("usb connect");
    let config = device.load_device_config().await.expect("load config");
    println!("{config:#?}");
    let _ = device.disconnect().await;
}
