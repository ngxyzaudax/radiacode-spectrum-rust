use uuid::Uuid;

pub const SERVICE: Uuid = Uuid::from_u128(0xe632_15e5_7003_49d8_96b0_b024_798f_b901);
pub const WRITE: Uuid = Uuid::from_u128(0xe632_15e6_7003_49d8_96b0_b024_798f_b901);
pub const NOTIFY: Uuid = Uuid::from_u128(0xe632_15e7_7003_49d8_96b0_b024_798f_b901);

pub const CHUNK_SIZE: usize = 18;
pub const RESPONSE_TIMEOUT_SECS: u64 = 30;
