//! # Thatch Roguelike
//!
//! A deep, complex roguelike game with LLM-driven dungeon mastering capabilities.
//!
//! ## Architecture Overview
//!
//! Thatch is designed with modularity and extensibility in mind. The core architecture
//! revolves around several key concepts:
//!
//! - **Game State**: Centralized state management for the entire game world
//! - **Entity System**: Flexible entity-component system for game objects
//! - **Action System**: Command pattern for all game actions (MCP-compatible)
//! - **Generation System**: Procedural content generation with LLM integration points
//! - **Rendering System**: Terminal-based rendering using crossterm
//!
//! ## MCP Integration
//!
//! The game is designed to be controllable via Model Context Protocol (MCP) for
//! future integration with LLM-based dungeon masters. All game actions are
//! serializable and can be executed remotely.

pub mod game;
pub mod generation;
pub mod input;
pub mod lldm;
pub mod rendering;
pub mod utils;

// Core module re-exports
pub use game::*;
pub use generation::*;
pub use input::*;
pub use lldm::*;
pub use rendering::*;
pub use utils::*;

// Explicit re-exports for commonly used types to ensure cross-platform compatibility
pub use game::{
    // From actions
    Action,
    ActionResult,
    ActionType,
    AttackAction,
    ConcreteAction,
    // From entities
    ConcreteEntity,
    Direction,
    Entity,
    EntityId,
    EntityStats,
    // From state
    GameCompletionState,
    GameEvent,
    GameState,
    GameTimeInfo,
    // From world
    Level,
    MessageImportance,
    MoveAction,
    PlayerCharacter,
    Position,
    StairDirection,
    Tile,
    TileType,
    UseStairsAction,
    WaitAction,
    World,
};

pub use generation::{
    GenerationConfig, Generator, Room, RoomCorridorGenerator, RoomType, WorldGenerator,
};

pub use rendering::{MacroquadDisplay, UI};

/// Core error type for the Thatch game engine.
#[derive(thiserror::Error, Debug)]
pub enum ThatchError {
    /// I/O operation failed
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    /// Game state is invalid
    #[error("Invalid game state: {0}")]
    InvalidState(String),

    /// Action cannot be performed
    #[error("Invalid action: {0}")]
    InvalidAction(String),

    /// Generation failed
    #[error("Generation failed: {0}")]
    GenerationFailed(String),

    /// LLM integration error
    #[error("LLDM error: {0}")]
    LldmError(String),
}

/// Result type used throughout the Thatch codebase.
pub type ThatchResult<T> = Result<T, ThatchError>;

/// Version information for the game.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Game configuration constants.
pub mod config {
    /// Default dungeon width in tiles
    pub const DEFAULT_DUNGEON_WIDTH: u32 = 80;

    /// Default dungeon height in tiles
    pub const DEFAULT_DUNGEON_HEIGHT: u32 = 40;

    /// Maximum number of entities per level
    pub const MAX_ENTITIES_PER_LEVEL: usize = 1000;

    /// Default player starting health
    pub const DEFAULT_PLAYER_HEALTH: u32 = 100;

    /// Frames per second target for the game loop
    pub const TARGET_FPS: u64 = 60;
}
