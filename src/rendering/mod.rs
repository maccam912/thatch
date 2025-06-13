//! # Rendering Module
//!
//! 2D graphics rendering system using macroquad for display management.

pub mod display;
pub mod ui;

pub use display::*;
pub use ui::*;

/// Placeholder rendering system for macroquad graphics output.
pub struct Renderer;

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderer {
    /// Creates a new renderer instance.
    pub fn new() -> Self {
        Self
    }
}
