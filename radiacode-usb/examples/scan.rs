fn main() {
    match radiacode_usb::scan_usb_devices() {
        Ok(devices) => {
            for device in devices {
                println!(
                    "{} {} {}",
                    device.transport_tag(),
                    device.display_label(),
                    device.endpoint.address_label()
                );
            }
        }
        Err(error) => eprintln!("usb scan failed: {error}"),
    }
}
