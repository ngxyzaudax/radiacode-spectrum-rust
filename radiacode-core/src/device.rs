use std::time::Duration;

use tracing::{debug, info, warn};

use crate::buffer::BytesBuffer;
use crate::command::{Command, VirtSfr, VirtString};
use crate::error::{Error, Result};
use crate::protocol::{build_request, request_header, strip_echoed_header, Sequence};
use crate::session_restore::SessionRestore;
use crate::transport::Transport;
use crate::types::DeviceVersions;

const CONNECT_ATTEMPTS: usize = 3;
const RECONNECT_ATTEMPTS: usize = 2;
const CONNECT_RETRY_DELAY: Duration = Duration::from_millis(1500);
const RECONNECT_RETRY_DELAY: Duration = Duration::from_millis(3000);
const INIT_SETTLE: Duration = Duration::from_millis(500);

struct OpenFailure {
    error: Error,
    transport: Box<dyn Transport>,
}

pub struct RadiaCode {
    pub(crate) transport: Box<dyn Transport>,
    pub(crate) sequence: Sequence,
    pub(crate) spectrum_format_version: u32,
    cached_versions: Option<DeviceVersions>,
}

impl RadiaCode {
    pub async fn open(
        transport: Box<dyn Transport>,
        ignore_firmware_check: bool,
        restore: Option<&SessionRestore>,
    ) -> Result<Self> {
        let reconnect = restore.is_some();
        let attempts = if reconnect {
            RECONNECT_ATTEMPTS
        } else {
            CONNECT_ATTEMPTS
        };
        let retry_delay = if reconnect {
            RECONNECT_RETRY_DELAY
        } else {
            CONNECT_RETRY_DELAY
        };
        info!(ignore_firmware_check, reconnect, "radiacode open");
        let mut last_error: Option<Error> = None;
        let mut transport = transport;
        for attempt in 0..attempts {
            if attempt > 0 {
                warn!(attempt, reconnect, "retrying radiacode open after transient failure");
                tokio::time::sleep(retry_delay).await;
            }
            match Self::try_open_once(transport, ignore_firmware_check, restore).await {
                Ok(device) => return Ok(device),
                Err(OpenFailure { error, transport: recovered }) if error.is_transient() => {
                    transport = recovered;
                    last_error = Some(error);
                }
                Err(OpenFailure { error, transport: recovered }) => {
                    let _ = recovered.disconnect().await;
                    return Err(error);
                }
            }
        }
        let _ = transport.disconnect().await;
        Err(last_error.unwrap_or(Error::ConnectionClosed))
    }

    async fn try_open_once(
        transport: Box<dyn Transport>,
        ignore_firmware_check: bool,
        restore: Option<&SessionRestore>,
    ) -> std::result::Result<Self, OpenFailure> {
        let sequence = Sequence::session_start();
        debug!(seq_start = sequence.start_value(), "command sequence initialized");
        let mut device = Self {
            transport,
            sequence,
            spectrum_format_version: restore
                .map(|session| session.spectrum_format_version)
                .unwrap_or(0),
            cached_versions: restore.map(|session| session.versions.clone()),
        };
        if let Err(error) = device
            .initialize(ignore_firmware_check, restore)
            .await
        {
            warn!(%error, "initialize failed, disconnecting partial session");
            let transport = device.transport;
            return Err(OpenFailure { error, transport });
        }
        Ok(device)
    }

    async fn initialize(
        &mut self,
        ignore_firmware_check: bool,
        restore: Option<&SessionRestore>,
    ) -> Result<()> {
        debug!(reconnect = restore.is_some(), "initializing radiacode session");
        self.execute_raw(Command::SetExchange, &[0x01, 0xff, 0x12, 0xff])
            .await?;
        self.drain_transport().await;
        tokio::time::sleep(INIT_SETTLE).await;
        self.drain_transport().await;
        crate::device_time::set_local_time_now(self).await?;
        self.write_vsfr(VirtSfr::DeviceTime, &0u32.to_le_bytes()).await?;
        self.drain_transport().await;
        tokio::time::sleep(INIT_SETTLE).await;
        self.drain_transport().await;

        if let Some(restore) = restore {
            self.cached_versions = Some(restore.versions.clone());
            self.spectrum_format_version = restore.spectrum_format_version;
            let major = restore.versions.target.major;
            let minor = restore.versions.target.minor;
            info!(
                major,
                minor,
                spectrum_format_version = self.spectrum_format_version,
                "reconnect session restored from cache"
            );
            return Ok(());
        }

        let versions = crate::device_info::fw_version(self).await?;
        self.cached_versions = Some(versions.clone());
        let major = versions.target.major;
        let minor = versions.target.minor;
        info!(major, minor, "device firmware");
        if !ignore_firmware_check && (major < 4 || (major == 4 && minor < 8)) {
            return Err(Error::IncompatibleFirmware { major, minor });
        }

        let configuration = crate::device_info::configuration(self).await?;
        self.spectrum_format_version = parse_spectrum_format_version(&configuration);
        info!(
            spectrum_format_version = self.spectrum_format_version,
            "device initialized"
        );
        Ok(())
    }

    pub async fn execute_raw(&mut self, command: Command, args: &[u8]) -> Result<BytesBuffer> {
        let seq = self.sequence.next();
        debug!(?command, seq, args_len = args.len(), "execute command");
        let request = build_request(command, seq, args);
        let response = self.transport.execute(&request).await?;
        strip_echoed_header(response, request_header(command, seq))
    }

    pub async fn read_virt_string(&mut self, id: VirtString) -> Result<BytesBuffer> {
        let mut response = self
            .execute_raw(Command::RdVirtString, &u32::from(id).to_le_bytes())
            .await?;
        let retcode = response.take_u32_le()?;
        let flen = response.take_u32_le()? as usize;
        if retcode != 1 {
            return Err(Error::UnexpectedReturnCode(retcode));
        }
        trim_trailing_nul_if_needed(&mut response, flen);
        if response.size() != flen {
            return Err(Error::BufferUnderrun {
                need: flen,
                have: response.size(),
            });
        }
        Ok(response)
    }

    pub async fn read_vsfr_u32(&mut self, id: VirtSfr) -> Result<u32> {
        let mut response = self
            .execute_raw(Command::RdVirtSfr, &u32::from(id).to_le_bytes())
            .await?;
        let retcode = response.take_u32_le()?;
        if retcode != 1 {
            return Err(Error::UnexpectedReturnCode(retcode));
        }
        response.take_u32_le()
    }

    pub async fn read_vsfr_optional(&mut self, id: VirtSfr) -> Result<Option<u32>> {
        let mut response = self
            .execute_raw(Command::RdVirtSfr, &u32::from(id).to_le_bytes())
            .await?;
        let retcode = response.take_u32_le()?;
        if retcode == 1 {
            Ok(Some(response.take_u32_le()?))
        } else if retcode == 0 {
            Ok(None)
        } else {
            Err(Error::UnexpectedReturnCode(retcode))
        }
    }

    pub async fn write_vsfr(&mut self, id: VirtSfr, data: &[u8]) -> Result<()> {
        if !self.write_vsfr_optional(id, data).await? {
            return Err(Error::UnexpectedReturnCode(0));
        }
        Ok(())
    }

    pub async fn write_vsfr_optional(&mut self, id: VirtSfr, data: &[u8]) -> Result<bool> {
        let mut args = u32::from(id).to_le_bytes().to_vec();
        args.extend_from_slice(data);
        let mut response = self.execute_raw(Command::WrVirtSfr, &args).await?;
        let retcode = response.take_u32_le()?;
        if retcode == 1 {
            if response.size() != 0 {
                return Err(Error::ProtocolMismatch {
                    expected: "empty payload".into(),
                    got: format!("{} trailing bytes", response.size()),
                });
            }
            Ok(true)
        } else if retcode == 0 {
            Ok(false)
        } else {
            Err(Error::UnexpectedReturnCode(retcode))
        }
    }

    pub async fn read_vsfr_f32(&mut self, id: VirtSfr) -> Result<f32> {
        let raw = self.read_vsfr_u32(id).await?;
        Ok(f32::from_le_bytes(raw.to_le_bytes()))
    }

    pub async fn read_vsfr_batch(&mut self, ids: &[VirtSfr]) -> Result<Vec<u32>> {
        crate::vsfr_batch::read_vsfr_batch(self, ids).await
    }

    pub async fn write_vsfr_batch(&mut self, pairs: &[(VirtSfr, u32)]) -> Result<()> {
        crate::vsfr_batch::write_vsfr_batch(self, pairs).await
    }

    pub async fn disconnect(self) -> Result<()> {
        self.transport.disconnect().await
    }

    pub async fn rssi_dbm(&self) -> Option<i16> {
        self.transport.link_rssi_dbm().await
    }

    pub async fn drain_transport(&mut self) {
        self.transport.drain_link().await;
    }

    pub async fn sample_rssi_dbm(&self) -> Option<i16> {
        self.transport.sample_link_rssi_dbm().await
    }

    pub(crate) fn cached_versions(&self) -> Option<&DeviceVersions> {
        self.cached_versions.as_ref()
    }

    pub fn session_restore(&self) -> Option<SessionRestore> {
        Some(SessionRestore {
            versions: self.cached_versions.as_ref()?.clone(),
            spectrum_format_version: self.spectrum_format_version,
        })
    }

    pub fn spectrum_format_version(&self) -> u32 {
        self.spectrum_format_version
    }
}

pub(crate) fn parse_spectrum_format_version(configuration: &str) -> u32 {
    configuration
        .lines()
        .find_map(|line| line.strip_prefix("SpecFormatVersion="))
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}

fn trim_trailing_nul_if_needed(buffer: &mut BytesBuffer, expected_len: usize) {
    let data = buffer.data();
    if data.len() == expected_len + 1 && data.last() == Some(&0) {
        let trimmed = data[..expected_len].to_vec();
        *buffer = BytesBuffer::new(trimmed);
    }
}
