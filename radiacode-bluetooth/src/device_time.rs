use crate::command::Command;
use crate::device::RadiaCode;
use crate::error::Result;

pub async fn set_local_time_now(device: &mut RadiaCode) -> Result<()> {
    let payload = local_time_payload(std::time::SystemTime::now())?;
    device.execute_raw(Command::SetTime, &payload).await?;
    Ok(())
}

fn local_time_payload(now: std::time::SystemTime) -> Result<[u8; 8]> {
    let datetime = time::OffsetDateTime::from(now).to_offset(local_offset());
    Ok([
        datetime.day(),
        datetime.month() as u8,
        (datetime.year() - 2000) as u8,
        0,
        datetime.second(),
        datetime.minute(),
        datetime.hour(),
        0,
    ])
}

fn local_offset() -> time::UtcOffset {
    time::UtcOffset::current_local_offset().unwrap_or(time::UtcOffset::UTC)
}
