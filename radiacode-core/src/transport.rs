use async_trait::async_trait;

use crate::buffer::BytesBuffer;
use crate::error::Result;

#[async_trait(?Send)]
pub trait Transport: Send {
    async fn execute(&mut self, request: &[u8]) -> Result<BytesBuffer>;
    async fn drain_link(&mut self);
    async fn disconnect(self: Box<Self>) -> Result<()>;
    async fn link_rssi_dbm(&self) -> Option<i16>;
    async fn sample_link_rssi_dbm(&self) -> Option<i16>;
}
