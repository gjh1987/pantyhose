pub mod protobuf;
pub mod message_traits;

// Re-export common traits for easy access
pub use message_traits::{MessageId, MessageIdSerialize};