use std::time::Duration;

use crate::buffer::BytesBuffer;
use crate::error::{Error, Result};
use crate::types::Spectrum;

pub fn decode_spectrum(buffer: &mut BytesBuffer, format_version: u32) -> Result<Spectrum> {
    let duration_sec = buffer.take_u32_le()?;
    let a0 = buffer.take_f32_le()?;
    let a1 = buffer.take_f32_le()?;
    let a2 = buffer.take_f32_le()?;
    let counts = match format_version {
        0 => decode_counts_v0(buffer)?,
        1 => decode_counts_v1(buffer)?,
        other => {
            return Err(Error::ProtocolMismatch {
                expected: "spectrum format 0 or 1".into(),
                got: format!("format {other}"),
            });
        }
    };
    Ok(Spectrum {
        duration: Duration::from_secs(duration_sec as u64),
        a0,
        a1,
        a2,
        counts,
    })
}

fn decode_counts_v0(buffer: &mut BytesBuffer) -> Result<Vec<u32>> {
    let mut counts = Vec::with_capacity(buffer.size() / 4);
    while buffer.size() >= 4 {
        counts.push(buffer.take_u32_le()?);
    }
    Ok(counts)
}

fn decode_counts_v1(buffer: &mut BytesBuffer) -> Result<Vec<u32>> {
    let mut counts = Vec::new();
    let mut last: i32 = 0;
    while buffer.size() >= 2 {
        let packed = buffer.take_u16_le()?;
        let repeat = ((packed >> 4) & 0x0FFF) as usize;
        let vlen = packed & 0x0F;
        for _ in 0..repeat {
            let value = read_compressed_count(buffer, vlen, last)?;
            last = value;
            counts.push(value as u32);
        }
    }
    Ok(counts)
}

fn read_compressed_count(buffer: &mut BytesBuffer, vlen: u16, last: i32) -> Result<i32> {
    match vlen {
        0 => Ok(0),
        1 => Ok(buffer.take_u8()? as i32),
        2 => Ok(last + take_i8(buffer)? as i32),
        3 => Ok(last + take_i16(buffer)? as i32),
        4 => {
            let a = buffer.take_u8()? as i32;
            let b = buffer.take_u8()? as i32;
            let c = take_i8(buffer)? as i32;
            Ok(last + ((c << 16) | (b << 8) | a))
        }
        5 => Ok(last + buffer.take_i32_le()?),
        other => Err(Error::ProtocolMismatch {
            expected: "vlen 0..=5".into(),
            got: format!("vlen {other}"),
        }),
    }
}

fn take_i8(buffer: &mut BytesBuffer) -> Result<i8> {
    Ok(buffer.take_u8()? as i8)
}

fn take_i16(buffer: &mut BytesBuffer) -> Result<i16> {
    let bytes = buffer.take_bytes(2)?;
    Ok(i16::from_le_bytes([bytes[0], bytes[1]]))
}
