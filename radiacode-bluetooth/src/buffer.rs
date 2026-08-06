use crate::error::{Error, Result};

#[derive(Debug, Clone)]
pub struct BytesBuffer {
    data: Vec<u8>,
    pos: usize,
}

impl BytesBuffer {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data, pos: 0 }
    }

    pub fn size(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    pub fn data(&self) -> &[u8] {
        &self.data[self.pos..]
    }

    pub fn take_u8(&mut self) -> Result<u8> {
        self.take_bytes(1).map(|b| b[0])
    }

    pub fn take_u16_le(&mut self) -> Result<u16> {
        let bytes = self.take_bytes(2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    pub fn take_u32_le(&mut self) -> Result<u32> {
        let bytes = self.take_bytes(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    pub fn take_i32_le(&mut self) -> Result<i32> {
        let bytes = self.take_bytes(4)?;
        Ok(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    pub fn take_f32_le(&mut self) -> Result<f32> {
        let bytes = self.take_bytes(4)?;
        Ok(f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    pub fn take_bytes(&mut self, count: usize) -> Result<&[u8]> {
        let have = self.size();
        if have < count {
            return Err(Error::BufferUnderrun { need: count, have });
        }
        let start = self.pos;
        self.pos += count;
        Ok(&self.data[start..self.pos])
    }

    pub fn take_length_prefixed_ascii(&mut self) -> Result<String> {
        let len = self.take_u8()? as usize;
        let bytes = self.take_bytes(len)?;
        Ok(String::from_utf8_lossy(bytes).into_owned())
    }

    pub fn skip(&mut self, count: usize) -> Result<()> {
        let _ = self.take_bytes(count)?;
        Ok(())
    }

    pub fn into_remaining(self) -> Vec<u8> {
        self.data[self.pos..].to_vec()
    }
}
