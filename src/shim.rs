//! Shim module to abstract over std and loom primitives.
//!
//! This module provides a unified interface for synchronization primitives that transparently
//! switches between `std` implementation (for production) and `loom` implementation (for testing).

#[cfg(not(feature = "loom"))]
pub mod atomic {
    pub use std::sync::atomic::*;
}

#[cfg(feature = "loom")]
pub mod atomic {
    pub use loom::sync::atomic::*;
}

#[cfg(not(feature = "loom"))]
pub mod sync {
    pub use std::sync::Arc;
}

#[cfg(feature = "loom")]
pub mod sync {
    pub use loom::sync::Arc;
}
