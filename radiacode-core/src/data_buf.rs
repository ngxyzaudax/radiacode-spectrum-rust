use crate::buffer::BytesBuffer;
use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RareStatus {
    pub duration_secs: u32,
    pub dose_r: f32,
    pub temperature_c: f32,
    pub battery_percent: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RealTimeRates {
    pub count_rate_cps: f32,
    pub dose_rate_rh: f32,
    pub count_rate_err_pct: f32,
    pub dose_rate_err_pct: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct DataBufSnapshot {
    pub rare: Option<RareStatus>,
    pub rates: Option<RealTimeRates>,
}

pub fn latest_rare_status(data: &[u8]) -> Option<RareStatus> {
    latest_snapshot(data).rare
}

pub fn latest_real_time_rates(data: &[u8]) -> Option<RealTimeRates> {
    latest_snapshot(data).rates
}

pub fn latest_snapshot(data: &[u8]) -> DataBufSnapshot {
    let mut buffer = BytesBuffer::new(data.to_vec());
    let mut snapshot = DataBufSnapshot::default();
    let mut best_rates: Option<(u8, u8, RealTimeRates)> = None;
    while buffer.size() >= 7 {
        let Ok(seq) = buffer.take_u8() else {
            break;
        };
        let Ok(eid) = buffer.take_u8() else {
            break;
        };
        let Ok(gid) = buffer.take_u8() else {
            break;
        };
        let Ok(_ts_offset) = buffer.take_i32_le() else {
            break;
        };
        match parse_record(&mut buffer, eid, gid) {
            Ok(RecordParse::Rare(status)) => snapshot.rare = Some(status),
            Ok(RecordParse::Rates(rates)) => {
                let replace = best_rates.as_ref().is_none_or(|(best_seq, best_gid, _)| {
                    seq > *best_seq
                        || (seq == *best_seq && rates_record_rank(gid) > rates_record_rank(*best_gid))
                });
                if replace {
                    best_rates = Some((seq, gid, rates));
                }
            }
            Ok(RecordParse::Skip) => {}
            Err(_) => break,
        }
    }
    snapshot.rates = best_rates.map(|(_, _, rates)| rates);
    snapshot
}

fn rates_record_rank(gid: u8) -> u8 {
    match gid {
        0 => 3,
        2 => 2,
        1 => 1,
        _ => 0,
    }
}

enum RecordParse {
    Skip,
    Rare(RareStatus),
    Rates(RealTimeRates),
}

fn parse_record(buffer: &mut BytesBuffer, eid: u8, gid: u8) -> Result<RecordParse> {
    if eid == 0 && gid == 0 {
        let count_rate = buffer.take_f32_le()?;
        let dose_rate = buffer.take_f32_le()?;
        let count_rate_err = buffer.take_u16_le()?;
        let dose_rate_err = buffer.take_u16_le()?;
        let _flags = buffer.take_u16_le()?;
        let _rt_flags = buffer.take_u8()?;
        return Ok(RecordParse::Rates(RealTimeRates {
            count_rate_cps: count_rate,
            dose_rate_rh: dose_rate,
            count_rate_err_pct: f32::from(count_rate_err) / 10.0,
            dose_rate_err_pct: f32::from(dose_rate_err) / 10.0,
        }));
    }
    if eid == 0 && gid == 1 {
        let count_rate = buffer.take_f32_le()?;
        let dose_rate = buffer.take_f32_le()?;
        return Ok(RecordParse::Rates(RealTimeRates {
            count_rate_cps: count_rate,
            dose_rate_rh: dose_rate,
            count_rate_err_pct: 0.0,
            dose_rate_err_pct: 0.0,
        }));
    }
    if eid == 0 && gid == 2 {
        let _count = buffer.take_u32_le()?;
        let count_rate = buffer.take_f32_le()?;
        let dose_rate = buffer.take_f32_le()?;
        let dose_rate_err = buffer.take_u16_le()?;
        let _flags = buffer.take_u16_le()?;
        return Ok(RecordParse::Rates(RealTimeRates {
            count_rate_cps: count_rate,
            dose_rate_rh: dose_rate,
            count_rate_err_pct: 0.0,
            dose_rate_err_pct: f32::from(dose_rate_err) / 10.0,
        }));
    }
    if eid == 0 && gid == 3 {
        let duration_secs = buffer.take_u32_le()?;
        let dose_r = buffer.take_f32_le()?;
        let temperature_raw = buffer.take_u16_le()?;
        let charge_raw = buffer.take_u16_le()?;
        let _flags = buffer.take_u16_le()?;
        return Ok(RecordParse::Rare(RareStatus {
            duration_secs,
            dose_r,
            temperature_c: (f32::from(temperature_raw) - 2000.0) / 100.0,
            battery_percent: f32::from(charge_raw) / 100.0,
        }));
    }
    if eid == 0 && (4..=5).contains(&gid) {
        buffer.skip(16)?;
        return Ok(RecordParse::Skip);
    }
    if eid == 0 && gid == 6 {
        buffer.skip(6)?;
        return Ok(RecordParse::Skip);
    }
    if eid == 0 && gid == 7 {
        buffer.skip(4)?;
        return Ok(RecordParse::Skip);
    }
    if eid == 0 && (8..=9).contains(&gid) {
        buffer.skip(6)?;
        return Ok(RecordParse::Skip);
    }
    if eid == 1 && (1..=3).contains(&gid) {
        let samples = buffer.take_u16_le()? as usize;
        let _sample_time = buffer.take_u32_le()?;
        let sample_bytes = match gid {
            1 => 8,
            2 => 16,
            _ => 14,
        };
        buffer.skip(samples * sample_bytes)?;
        return Ok(RecordParse::Skip);
    }
    Err(Error::ProtocolMismatch {
        expected: "known data_buf record".into(),
        got: format!("eid={eid} gid={gid}"),
    })
}

#[cfg(test)]
mod tests {
    use super::{latest_rare_status, latest_real_time_rates, latest_snapshot};

    #[test]
    fn parses_rare_data_record() {
        let bytes = [
            1u8, 0, 3, 0, 0, 0, 0, 10, 0, 0, 0, 0, 0, 0, 0, 0xd4, 0x10, 0x34, 0x21, 0x00, 0x00,
        ];
        let status = latest_rare_status(&bytes).expect("rare");
        assert_eq!(status.duration_secs, 10);
        assert!((status.dose_r - 0.0).abs() < 0.001);
        assert!((status.temperature_c - 23.08).abs() < 0.01);
        assert!((status.battery_percent - 85.0).abs() < 0.01);
    }

    #[test]
    fn parses_real_time_data_record() {
        let count_rate = 12.5f32.to_le_bytes();
        let dose_rate = 0.000_125f32.to_le_bytes();
        let mut bytes = vec![1u8, 0, 0, 0, 0, 0, 0];
        bytes.extend_from_slice(&count_rate);
        bytes.extend_from_slice(&dose_rate);
        bytes.extend_from_slice(&[10u8, 0, 20, 0, 0, 0, 0]);
        let rates = latest_real_time_rates(&bytes).expect("rates");
        assert!((rates.count_rate_cps - 12.5).abs() < 0.001);
        assert!((rates.dose_rate_rh - 0.000_125).abs() < 1e-9);
        assert!((rates.count_rate_err_pct - 1.0).abs() < 0.001);
        assert!((rates.dose_rate_err_pct - 2.0).abs() < 0.001);
    }

    #[test]
    fn snapshot_prefers_latest_records() {
        let count_rate = 5.0f32.to_le_bytes();
        let dose_rate = 0.001f32.to_le_bytes();
        let mut bytes = vec![1u8, 0, 0, 0, 0, 0, 0];
        bytes.extend_from_slice(&count_rate);
        bytes.extend_from_slice(&dose_rate);
        bytes.extend_from_slice(&[0u8; 7]);
        bytes.extend_from_slice(&[
            2, 0, 3, 0, 0, 0, 0, 10, 0, 0, 0, 0, 0, 0, 0, 0xd4, 0x10, 0x34, 0x21, 0x00, 0x00,
        ]);
        let snapshot = latest_snapshot(&bytes);
        assert!(snapshot.rates.is_some());
        assert!(snapshot.rare.is_some());
    }

    #[test]
    fn snapshot_finds_rare_after_sequence_gap() {
        let count_rate = 5.0f32.to_le_bytes();
        let dose_rate = 0.001f32.to_le_bytes();
        let mut bytes = vec![9u8, 0, 0, 0, 0, 0, 0];
        bytes.extend_from_slice(&count_rate);
        bytes.extend_from_slice(&dose_rate);
        bytes.extend_from_slice(&[0u8; 7]);
        bytes.extend_from_slice(&[
            2, 0, 3, 0, 0, 0, 0, 10, 0, 0, 0, 0, 0, 0, 0, 0xd4, 0x10, 0x34, 0x21, 0x00, 0x00,
        ]);
        let snapshot = latest_snapshot(&bytes);
        assert!(snapshot.rates.is_some());
        assert!(snapshot.rare.is_some());
    }
}
