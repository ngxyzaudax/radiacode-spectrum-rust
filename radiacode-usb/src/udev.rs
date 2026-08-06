use std::fs;
use std::process::Command;

use crate::constants::{PID, VID};
use crate::transport::usb_permission_denied;
use rusb::{Context, UsbContext};

pub const RULES: &str = "SUBSYSTEM==\"usb\", ATTR{idVendor}==\"0483\", ATTR{idProduct}==\"f123\", MODE=\"0666\", TAG+=\"uaccess\"\n";
pub const RULE_DEST: &str = "/etc/udev/rules.d/99-radiacode.rules";
const TEMP_RULE: &str = "/tmp/radiacode-udev.rules";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsbAccessStatus {
    Granted,
    RuleMissing,
    RuleInstalledNeedReplug,
}

pub fn access_status() -> UsbAccessStatus {
    if !device_present() {
        return UsbAccessStatus::Granted;
    }
    if !usb_permission_denied() {
        return UsbAccessStatus::Granted;
    }
    if rule_installed() {
        UsbAccessStatus::RuleInstalledNeedReplug
    } else {
        UsbAccessStatus::RuleMissing
    }
}

pub fn rule_installed() -> bool {
    fs::read_to_string(RULE_DEST)
        .ok()
        .is_some_and(|content| {
            content.contains("0483")
                && content.contains("f123")
                && content.contains("0666")
        })
}

pub fn install_access_rule() -> Result<(), String> {
    fs::write(TEMP_RULE, RULES).map_err(|error| error.to_string())?;
    let script = format!(
        "cp '{}' '{}' && udevadm control --reload && udevadm trigger",
        TEMP_RULE, RULE_DEST
    );
    run_pkexec(&script)
}

fn device_present() -> bool {
    let Ok(context) = Context::new() else {
        return false;
    };
    let Ok(devices) = context.devices() else {
        return false;
    };
    devices.iter().any(|device| {
        device
            .device_descriptor()
            .ok()
            .is_some_and(|descriptor| descriptor.vendor_id() == VID && descriptor.product_id() == PID)
    })
}

fn run_pkexec(script: &str) -> Result<(), String> {
    let mut command = Command::new("pkexec");
    command.args(["sh", "-c", script]);
    if let Ok(display) = std::env::var("DISPLAY") {
        command.env("DISPLAY", display);
    }
    if let Ok(xauthority) = std::env::var("XAUTHORITY") {
        command.env("XAUTHORITY", xauthority);
    }
    let status = command
        .status()
        .map_err(|error| format!("pkexec unavailable: {error}"))?;
    if status.success() {
        return Ok(());
    }
    let code = status.code().unwrap_or(-1);
    if code == 126 || code == 127 {
        return Err("Authentication cancelled.".into());
    }
    Err(format!("Privileged setup failed with exit code {code}."))
}
