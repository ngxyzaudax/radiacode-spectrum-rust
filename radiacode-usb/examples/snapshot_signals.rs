use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::PathBuf;

use radiacode_core::Command;
use radiacode_usb::{connect, scan_usb_devices};

const SAFE_IDS: &[(u32, &str)] = &[
    (0x0500, "DEVICE_CTRL"),
    (0x0502, "DEVICE_LANG"),
    (0x0503, "DEVICE_ON"),
    (0x0510, "DISP_CTRL"),
    (0x0511, "DISP_BRT"),
    (0x0512, "DISP_CONTR"),
    (0x0513, "DISP_OFF_TIME"),
    (0x0514, "DISP_ON"),
    (0x0515, "DISP_DIR"),
    (0x0516, "DISP_BACKLT_ON"),
    (0x0520, "SOUND_CTRL"),
    (0x0521, "SOUND_VOL"),
    (0x0522, "SOUND_ON"),
    (0x0523, "SOUND_BUTTON"),
    (0x0530, "VIBRO_CTRL"),
    (0x0531, "VIBRO_ON"),
    (0x0540, "LEDS_CTRL"),
    (0x0541, "LED0_BRT"),
    (0x0542, "LED1_BRT"),
    (0x0543, "LED2_BRT"),
    (0x0544, "LED3_BRT"),
    (0x0545, "LEDS_ON"),
    (0x05E0, "ALARM_MODE"),
    (0x05E1, "PLAY_SIGNAL"),
    (0x0700, "BLE_TX_PWR"),
    (0x8004, "DS_UNITS"),
    (0x800C, "USE_NSV_H"),
    (0x8013, "CR_UNITS"),
];

async fn read_optional(device: &mut radiacode_core::RadiaCode, id: u32) -> Option<u32> {
    let mut response = device
        .execute_raw(Command::RdVirtSfr, &id.to_le_bytes())
        .await
        .ok()?;
    let ret = response.take_u32_le().ok()?;
    if ret == 1 && response.size() >= 4 {
        response.take_u32_le().ok()
    } else {
        None
    }
}

async fn snapshot(device: &mut radiacode_core::RadiaCode) -> BTreeMap<u32, Option<u32>> {
    let mut values = BTreeMap::new();
    for (id, _) in SAFE_IDS {
        values.insert(*id, read_optional(device, *id).await);
    }
    values
}

fn label(id: u32) -> &'static str {
    SAFE_IDS
        .iter()
        .find(|(candidate, _)| *candidate == id)
        .map(|(_, name)| *name)
        .unwrap_or("UNKNOWN")
}

fn encode(values: &BTreeMap<u32, Option<u32>>) -> String {
    let mut lines = Vec::new();
    for (id, value) in values {
        let rendered = match value {
            Some(raw) => format!("0x{raw:08X}"),
            None => "NONE".into(),
        };
        lines.push(format!("0x{id:04X}\t{}\t{rendered}", label(*id)));
    }
    lines.join("\n") + "\n"
}

fn decode(text: &str) -> BTreeMap<u32, Option<u32>> {
    let mut values = BTreeMap::new();
    for line in text.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() != 3 {
            continue;
        }
        let id = u32::from_str_radix(parts[0].trim_start_matches("0x"), 16).unwrap_or(0);
        let value = if parts[2] == "NONE" {
            None
        } else {
            u32::from_str_radix(parts[2].trim_start_matches("0x"), 16).ok()
        };
        values.insert(id, value);
    }
    values
}

fn default_path(name: &str) -> PathBuf {
    PathBuf::from(format!("/tmp/radiacode_signals_{name}.txt"))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let mode = args.get(1).map(String::as_str).unwrap_or("help");
    match mode {
        "save" => {
            let name = args.get(2).map(String::as_str).unwrap_or("before");
            let path = default_path(name);
            let devices = scan_usb_devices()?;
            let mut device = connect(&devices[0].endpoint.address_label()).await?;
            let serial = device.serial_number().await?;
            let values = snapshot(&mut device).await;
            fs::write(&path, encode(&values))?;
            println!("saved {path:?} for {serial} ({} ids)", values.len());
            for (id, value) in &values {
                println!("{:16} 0x{id:04X} = {value:?}", label(*id));
            }
            let _ = device.disconnect().await;
        }
        "diff" => {
            let left_name = args.get(2).map(String::as_str).unwrap_or("before");
            let right_name = args.get(3).map(String::as_str).unwrap_or("after");
            let left = decode(&fs::read_to_string(default_path(left_name))?);
            let right = decode(&fs::read_to_string(default_path(right_name))?);
            let mut changed = 0usize;
            for id in left.keys().chain(right.keys()) {
                let a = left.get(id).copied().flatten();
                let b = right.get(id).copied().flatten();
                if a != b {
                    changed += 1;
                    println!("CHANGED {:16} 0x{id:04X}: {a:?} -> {b:?}", label(*id));
                }
            }
            if changed == 0 {
                println!("No differences between {left_name} and {right_name}.");
            } else {
                println!("{changed} register(s) changed.");
            }
        }
        _ => {
            println!("usage:");
            println!("  snapshot_signals save before|after");
            println!("  snapshot_signals diff before after");
        }
    }
    Ok(())
}
