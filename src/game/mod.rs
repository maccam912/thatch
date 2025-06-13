//! # Game Module
//!
//! Core game state management, world representation, and entity systems.
//!
//! This module contains the fundamental building blocks of the Thatch roguelike:
//! - Game state management and persistence
//! - World and level representation
//! - Entity-component system for game objects
//! - Action system for MCP-compatible commands

pub mod actions;
pub mod entities;
pub mod state;
pub mod world;

pub use actions::*;
pub use entities::*;
pub use state::*;
pub use world::*;

use crate::{ThatchError, ThatchResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Represents a 2D coordinate in the game world.
///
/// # Examples
///
/// ```
/// use thatch::Position;
///
/// let pos = Position::new(10, 5);
/// assert_eq!(pos.x, 10);
/// assert_eq!(pos.y, 5);
///
/// let adjacent = pos.adjacent_positions();
/// assert_eq!(adjacent.len(), 8); // All 8 surrounding positions
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

impl Position {
    /// Creates a new position with the given coordinates.
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Returns the origin position (0, 0).
    pub fn origin() -> Self {
        Self::new(0, 0)
    }

    /// Calculates the Manhattan distance to another position.
    ///
    /// # Examples
    ///
    /// ```
    /// use thatch::Position;
    ///
    /// let pos1 = Position::new(0, 0);
    /// let pos2 = Position::new(3, 4);
    /// assert_eq!(pos1.manhattan_distance(pos2), 7);
    /// ```
    pub fn manhattan_distance(self, other: Position) -> u32 {
        ((self.x - other.x).abs() + (self.y - other.y).abs()) as u32
    }

    /// Calculates the Euclidean distance to another position.
    pub fn euclidean_distance(self, other: Position) -> f64 {
        let dx = (self.x - other.x) as f64;
        let dy = (self.y - other.y) as f64;
        (dx * dx + dy * dy).sqrt()
    }

    /// Returns all 8 adjacent positions (including diagonals).
    pub fn adjacent_positions(self) -> Vec<Position> {
        vec![
            Position::new(self.x - 1, self.y - 1), // NW
            Position::new(self.x, self.y - 1),     // N
            Position::new(self.x + 1, self.y - 1), // NE
            Position::new(self.x - 1, self.y),     // W
            Position::new(self.x + 1, self.y),     // E
            Position::new(self.x - 1, self.y + 1), // SW
            Position::new(self.x, self.y + 1),     // S
            Position::new(self.x + 1, self.y + 1), // SE
        ]
    }

    /// Returns only the 4 cardinal adjacent positions (no diagonals).
    pub fn cardinal_adjacent_positions(self) -> Vec<Position> {
        vec![
            Position::new(self.x, self.y - 1), // N
            Position::new(self.x - 1, self.y), // W
            Position::new(self.x + 1, self.y), // E
            Position::new(self.x, self.y + 1), // S
        ]
    }
}

impl std::ops::Add for Position {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y)
    }
}

impl std::ops::Sub for Position {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y)
    }
}

/// Directions for movement and orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Direction {
    North,
    South,
    East,
    West,
    Northeast,
    Northwest,
    Southeast,
    Southwest,
}

impl Direction {
    /// Converts a direction to a position delta.
    ///
    /// # Examples
    ///
    /// ```
    /// use thatch::{Direction, Position};
    ///
    /// let delta = Direction::North.to_delta();
    /// assert_eq!(delta, Position::new(0, -1));
    /// ```
    pub fn to_delta(self) -> Position {
        match self {
            Direction::North => Position::new(0, -1),
            Direction::South => Position::new(0, 1),
            Direction::East => Position::new(1, 0),
            Direction::West => Position::new(-1, 0),
            Direction::Northeast => Position::new(1, -1),
            Direction::Northwest => Position::new(-1, -1),
            Direction::Southeast => Position::new(1, 1),
            Direction::Southwest => Position::new(-1, 1),
        }
    }

    /// Converts a position delta to a direction.
    ///
    /// Returns None if the delta doesn't correspond to a valid direction.
    pub fn from_delta(delta: Position) -> Option<Direction> {
        match (delta.x, delta.y) {
            (0, -1) => Some(Direction::North),
            (0, 1) => Some(Direction::South),
            (1, 0) => Some(Direction::East),
            (-1, 0) => Some(Direction::West),
            (1, -1) => Some(Direction::Northeast),
            (-1, -1) => Some(Direction::Northwest),
            (1, 1) => Some(Direction::Southeast),
            (-1, 1) => Some(Direction::Southwest),
            _ => None,
        }
    }

    /// Returns all 8 directions.
    pub fn all() -> Vec<Direction> {
        vec![
            Direction::North,
            Direction::South,
            Direction::East,
            Direction::West,
            Direction::Northeast,
            Direction::Northwest,
            Direction::Southeast,
            Direction::Southwest,
        ]
    }

    /// Returns only the 4 cardinal directions.
    pub fn cardinal() -> Vec<Direction> {
        vec![
            Direction::North,
            Direction::South,
            Direction::East,
            Direction::West,
        ]
    }
}

/// Unique identifier for game entities.
pub type EntityId = Uuid;

/// Creates a new unique entity ID.
pub fn new_entity_id() -> EntityId {
    Uuid::new_v4()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_creation() {
        let pos = Position::new(5, 10);
        assert_eq!(pos.x, 5);
        assert_eq!(pos.y, 10);
    }

    #[test]
    fn test_position_manhattan_distance() {
        let pos1 = Position::new(0, 0);
        let pos2 = Position::new(3, 4);
        assert_eq!(pos1.manhattan_distance(pos2), 7);
    }

    #[test]
    fn test_position_euclidean_distance() {
        let pos1 = Position::new(0, 0);
        let pos2 = Position::new(3, 4);
        assert_eq!(pos1.euclidean_distance(pos2), 5.0);
    }

    #[test]
    fn test_position_adjacent() {
        let pos = Position::new(5, 5);
        let adjacent = pos.adjacent_positions();
        assert_eq!(adjacent.len(), 8);
        assert!(adjacent.contains(&Position::new(4, 4)));
        assert!(adjacent.contains(&Position::new(6, 6)));
    }

    #[test]
    fn test_position_cardinal_adjacent() {
        let pos = Position::new(5, 5);
        let adjacent = pos.cardinal_adjacent_positions();
        assert_eq!(adjacent.len(), 4);
        assert!(adjacent.contains(&Position::new(5, 4))); // North
        assert!(adjacent.contains(&Position::new(4, 5))); // West
        assert!(!adjacent.contains(&Position::new(4, 4))); // No diagonal
    }

    #[test]
    fn test_position_arithmetic() {
        let pos1 = Position::new(5, 10);
        let pos2 = Position::new(3, 2);
        assert_eq!(pos1 + pos2, Position::new(8, 12));
        assert_eq!(pos1 - pos2, Position::new(2, 8));
    }

    #[test]
    fn test_direction_to_delta() {
        assert_eq!(Direction::North.to_delta(), Position::new(0, -1));
        assert_eq!(Direction::East.to_delta(), Position::new(1, 0));
        assert_eq!(Direction::Northeast.to_delta(), Position::new(1, -1));
    }

    #[test]
    fn test_entity_id_uniqueness() {
        let id1 = new_entity_id();
        let id2 = new_entity_id();
        assert_ne!(id1, id2);
    }
}
