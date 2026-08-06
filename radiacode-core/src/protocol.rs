use crate::buffer::BytesBuffer;
use crate::command::Command;
use crate::error::{Error, Result};

#[derive(Debug, Default)]
pub struct Sequence {
    value: u8,
}

impl Sequence {
    pub fn session_start() -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.subsec_nanos())
            .unwrap_or(0);
        Self {
            value: (seed as u8) % 32,
        }
    }

    pub fn start_value(&self) -> u8 {
        self.value
    }

    pub fn next(&mut self) -> u8 {
        let seq = 0x80 + self.value;
        self.value = (self.value + 1) % 32;
        seq
    }
}

pub fn build_request(command: Command, seq: u8, args: &[u8]) -> Vec<u8> {
    let mut request = Vec::with_capacity(4 + 4 + args.len());
    request.extend_from_slice(&u16::from(command).to_le_bytes());
    request.push(0);
    request.push(seq);
    request.extend_from_slice(args);
    let mut framed = Vec::with_capacity(4 + request.len());
    framed.extend_from_slice(&(request.len() as u32).to_le_bytes());
    framed.extend_from_slice(&request);
    framed
}

pub fn request_header(command: Command, seq: u8) -> [u8; 4] {
    let cmd = u16::from(command).to_le_bytes();
    [cmd[0], cmd[1], 0, seq]
}

pub fn framed_request_header(request: &[u8]) -> Result<[u8; 4]> {
    if request.len() < 8 {
        return Err(Error::ProtocolMismatch {
            expected: "framed request".into(),
            got: format!("{} bytes", request.len()),
        });
    }
    Ok([request[4], request[5], request[6], request[7]])
}

pub fn response_matches_request(payload: &[u8], expected: [u8; 4]) -> bool {
    payload.len() >= 4 && payload[0..4] == expected
}

pub fn strip_echoed_header(mut response: BytesBuffer, expected: [u8; 4]) -> Result<BytesBuffer> {
    let header = response.take_bytes(4)?;
    if header != expected {
        return Err(Error::ProtocolMismatch {
            expected: hex_bytes(&expected),
            got: hex_bytes(header),
        });
    }
    Ok(BytesBuffer::new(response.into_remaining()))
}

fn hex_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join("")
}

#[derive(Debug, Default)]
pub struct ResponseAssembler {
    remaining: usize,
    payload: Vec<u8>,
}

impl ResponseAssembler {
    pub fn push(&mut self, chunk: &[u8]) -> Result<Option<Vec<u8>>> {
        if self.remaining == 0 {
            if chunk.len() < 4 {
                return Err(Error::BufferUnderrun {
                    need: 4,
                    have: chunk.len(),
                });
            }
            let payload_len = i32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]) as usize;
            self.remaining = 4 + payload_len;
            self.payload.clear();
            self.payload.extend_from_slice(&chunk[4..]);
        } else {
            self.payload.extend_from_slice(chunk);
        }

        self.remaining = self.remaining.saturating_sub(chunk.len());
        if self.remaining == 0 {
            return Ok(Some(std::mem::take(&mut self.payload)));
        }
        Ok(None)
    }
}
