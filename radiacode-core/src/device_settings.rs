use crate::command::{Command, VirtSfr, VirtString};
use crate::device::RadiaCode;
use crate::error::{Error, Result};

impl RadiaCode {
    pub async fn spectrum_reset(&mut self) -> Result<()> {
        let mut args = Vec::with_capacity(8);
        args.extend_from_slice(&u32::from(VirtString::Spectrum).to_le_bytes());
        args.extend_from_slice(&0u32.to_le_bytes());
        let mut response = self.execute_raw(Command::WrVirtString, &args).await?;
        let retcode = response.take_u32_le()?;
        if retcode != 1 {
            return Err(Error::UnexpectedReturnCode(retcode));
        }
        Ok(())
    }

    pub async fn dose_reset(&mut self) -> Result<()> {
        self.write_vsfr(VirtSfr::DoseReset, &[]).await
    }

    pub async fn set_sound_on(&mut self, on: bool) -> Result<()> {
        self.write_vsfr(VirtSfr::SoundOn, &u32::from(on).to_le_bytes())
            .await
    }

    pub async fn set_vibro_on(&mut self, on: bool) -> Result<()> {
        self.write_vsfr(VirtSfr::VibroOn, &u32::from(on).to_le_bytes())
            .await
    }

    pub async fn set_display_brightness(&mut self, brightness: u8) -> Result<()> {
        if brightness > 9 {
            return Err(Error::ProtocolMismatch {
                expected: "brightness 0..=9".into(),
                got: brightness.to_string(),
            });
        }
        self.write_vsfr(VirtSfr::DispBrt, &(brightness as u32).to_le_bytes())
            .await
    }
}
