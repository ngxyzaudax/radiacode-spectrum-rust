use radiacode_usb::connect;

#[tokio::main]
async fn main() {
    let serial = std::env::args().nth(1).expect("usage: connect <serial-or-usb-id>");
    match connect(&serial).await {
        Ok(mut device) => {
            let metadata = device.metadata().await.expect("metadata");
            println!(
                "connected {} {} firmware {}",
                metadata.model,
                metadata.serial,
                metadata.firmware_label()
            );
            let _ = device.disconnect().await;
        }
        Err(error) => eprintln!("connect failed: {error}"),
    }
}
