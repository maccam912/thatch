//! # LLDM Module
//!
//! LLM Dungeon Master integration for enhanced content generation.

pub mod mcp;
pub mod traits;

pub use mcp::*;
pub use traits::*;

/// Placeholder for LLDM integration.
pub struct LldmClient;

impl LldmClient {
    /// Creates a new LLDM client.
    pub fn new() -> Self {
        Self
    }
}
