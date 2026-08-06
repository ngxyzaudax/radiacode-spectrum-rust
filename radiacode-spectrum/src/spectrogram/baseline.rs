use std::time::Instant;

#[derive(Debug, Clone)]
pub struct IngestBaseline {
    pub counts: Vec<u32>,
    pub device_duration_secs: f64,
    pub ingested_at: Instant,
}

impl IngestBaseline {
    pub fn new(counts: Vec<u32>, device_duration_secs: f64) -> Self {
        Self {
            counts,
            device_duration_secs,
            ingested_at: Instant::now(),
        }
    }
}
