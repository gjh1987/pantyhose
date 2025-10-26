// Message traits for protobuf serialization
// This file defines common traits that can be used across different protobuf implementations

use bytes::BytesMut;

/// Trait for messages with an ID
pub trait MessageId {
    fn msg_id(&self) -> u16;
}

/// Helper trait for serializable messages with ID
pub trait MessageIdSerialize: MessageId + prost::Message + Default {
    /// Serialize message to buffer with message ID and length header
    fn serialize_to_buffer(&self) -> Result<BytesMut, Box<dyn std::error::Error + Send + Sync>>;
}