use std::time::Duration;

use tokio::time::sleep;
use tracing::debug;

use crate::buffer::BytesBuffer;
use crate::command::{Command, VirtString};
use crate::device::RadiaCode;
use crate::error::{Error, Result};
use crate::spectrum::decode_spectrum;
use crate::types::{DeviceVersions, FirmwareVersion, Spectrum};

const FW_VERSION_ATTEMPTS: usize = 3;

pub fn decode_fw_version(response: BytesBuffer) -> Result<DeviceVersions> {
    let candidates = decode_fw_version_candidates(response.clone());
    for candidate in candidates {
        if let Ok(versions) = decode_fw_version_body(candidate) {
            return Ok(versions);
        }
    }
    decode_fw_version_body(response)
}

fn decode_fw_version_candidates(mut response: BytesBuffer) -> Vec<BytesBuffer> {
    let mut candidates = vec![response.clone()];
    while response.size() > 51 && response.data().last() == Some(&0) {
        let trimmed = response.data()[..response.size() - 1].to_vec();
        response = BytesBuffer::new(trimmed);
        candidates.push(response.clone());
    }
    if response.size() >= 4 {
        let mut skip_retcode = response.clone();
        if skip_retcode.take_u32_le().ok() == Some(1) {
            candidates.push(skip_retcode);
        }
    }
    candidates
}

fn decode_fw_version_body(mut response: BytesBuffer) -> Result<DeviceVersions> {
    let boot_minor = response.take_u16_le()?;
    let boot_major = response.take_u16_le()?;
    let boot_date = response.take_length_prefixed_ascii()?;
    let target_minor = response.take_u16_le()?;
    let target_major = response.take_u16_le()?;
    let target_date = response
        .take_length_prefixed_ascii()?
        .trim_end_matches('\0')
        .to_string();
    if response.size() != 0 {
        return Err(Error::ProtocolMismatch {
            expected: "empty fw_version tail".into(),
            got: format!("{} trailing bytes", response.size()),
        });
    }
    Ok(DeviceVersions {
        boot: FirmwareVersion {
            major: boot_major,
            minor: boot_minor,
            date: boot_date,
        },
        target: FirmwareVersion {
            major: target_major,
            minor: target_minor,
            date: target_date,
        },
    })
}

pub async fn fw_version(device: &mut RadiaCode) -> Result<DeviceVersions> {
    let mut last_error: Option<Error> = None;
    for attempt in 0..FW_VERSION_ATTEMPTS {
        if attempt > 0 {
            debug!(attempt, "retrying fw_version after transient parse error");
            sleep(Duration::from_millis(250)).await;
        }
        device.drain_transport().await;
        let response = device.execute_raw(Command::GetVersion, &[]).await?;
        match decode_fw_version(response) {
            Ok(versions) => return Ok(versions),
            Err(error) if error.is_transient() => last_error = Some(error),
            Err(error) => return Err(error),
        }
    }
    Err(last_error.unwrap_or(Error::Timeout))
}

pub async fn hw_serial_number(device: &mut RadiaCode) -> Result<String> {
    let mut response = device.execute_raw(Command::GetSerial, &[]).await?;
    let serial_len = response.take_u32_le()? as usize;
    let mut groups = Vec::new();
    for _ in 0..(serial_len / 4) {
        groups.push(format!("{:08X}", response.take_u32_le()?));
    }
    Ok(groups.join("-"))
}

pub async fn serial_number(device: &mut RadiaCode) -> Result<String> {
    let response = device.read_virt_string(VirtString::SerialNumber).await?;
    Ok(String::from_utf8_lossy(response.data()).into_owned())
}

pub async fn configuration(device: &mut RadiaCode) -> Result<String> {
    let response = device.read_virt_string(VirtString::Configuration).await?;
    Ok(String::from_utf8_lossy(response.data()).into_owned())
}

pub async fn spectrum(device: &mut RadiaCode) -> Result<Spectrum> {
    let mut response = device.read_virt_string(VirtString::Spectrum).await?;
    decode_spectrum(&mut response, device.spectrum_format_version)
}

pub async fn spectrum_accum(device: &mut RadiaCode) -> Result<Spectrum> {
    let mut response = device.read_virt_string(VirtString::SpecAccum).await?;
    decode_spectrum(&mut response, device.spectrum_format_version)
}

pub async fn energy_calib(device: &mut RadiaCode) -> Result<[f32; 3]> {
    let mut response = device.read_virt_string(VirtString::EnergyCalib).await?;
    Ok([
        response.take_f32_le()?,
        response.take_f32_le()?,
        response.take_f32_le()?,
    ])
}

impl RadiaCode {
    pub async fn fw_version(&mut self) -> Result<DeviceVersions> {
        fw_version(self).await
    }

    pub async fn hw_serial_number(&mut self) -> Result<String> {
        hw_serial_number(self).await
    }

    pub async fn serial_number(&mut self) -> Result<String> {
        serial_number(self).await
    }

    pub async fn configuration(&mut self) -> Result<String> {
        configuration(self).await
    }

    pub async fn spectrum(&mut self) -> Result<Spectrum> {
        spectrum(self).await
    }

    pub async fn spectrum_accum(&mut self) -> Result<Spectrum> {
        spectrum_accum(self).await
    }

    pub async fn energy_calib(&mut self) -> Result<[f32; 3]> {
        energy_calib(self).await
    }
}

#[cfg(test)]
mod tests {
    use super::decode_fw_version;
    use crate::buffer::BytesBuffer;

    fn ascii_field(value: &str) -> Vec<u8> {
        let mut bytes = vec![value.len() as u8];
        bytes.extend_from_slice(value.as_bytes());
        bytes
    }

    #[test]
    fn decode_fw_version_parses_boot_and_target() {
        let mut payload = Vec::new();
        payload.extend_from_slice(&4u16.to_le_bytes());
        payload.extend_from_slice(&5u16.to_le_bytes());
        payload.extend(ascii_field("boot-date"));
        payload.extend_from_slice(&8u16.to_le_bytes());
        payload.extend_from_slice(&4u16.to_le_bytes());
        payload.extend(ascii_field("target-date"));
        let versions = decode_fw_version(BytesBuffer::new(payload)).unwrap();
        assert_eq!(versions.boot.major, 5);
        assert_eq!(versions.boot.minor, 4);
        assert_eq!(versions.boot.date, "boot-date");
        assert_eq!(versions.target.major, 4);
        assert_eq!(versions.target.minor, 8);
        assert_eq!(versions.target.date, "target-date");
    }
}
