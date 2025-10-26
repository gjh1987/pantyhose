//! Data structures and utilities module
//! 
//! This module provides various data structures and utilities for the framework,
//! including dynamic buffers for efficient memory management and heap data structures.

pub mod dynamic_buffer;
pub mod min_heap;
pub mod max_heap;

// Re-export the main types
pub use dynamic_buffer::{DynamicBuffer, BufferStats};
pub use min_heap::MinHeap;
pub use max_heap::MaxHeap;