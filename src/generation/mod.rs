//! # Generation Module
//!
//! Procedural content generation systems for dungeons, items, and encounters.
//!
//! This module provides the foundation for creating procedural content in Thatch.
//! It includes dungeon layout generation, item creation, and encounter placement.
//! The system is designed to integrate with the LLDM for enhanced content generation.

pub mod dungeon;
pub mod encounters;
pub mod items;

pub use dungeon::*;
pub use encounters::*;
pub use items::*;

use crate::{ThatchError, ThatchResult};
use crate::game::{Level, Position, TileType};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for procedural generation.
///
/// Controls various aspects of how content is generated, including
/// randomness parameters, density settings, and LLDM integration options.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationConfig {
    /// Random seed for reproducible generation
    pub seed: u64,
    /// Minimum room size
    pub min_room_size: u32,
    /// Maximum room size
    pub max_room_size: u32,
    /// Minimum number of rooms per level
    pub min_rooms: u32,
    /// Maximum number of rooms per level
    pub max_rooms: u32,
    /// Corridor width
    pub corridor_width: u32,
    /// Probability of extra connections between rooms (0.0 to 1.0)
    pub extra_connection_chance: f64,
    /// Probability of secret doors (0.0 to 1.0)
    pub secret_door_chance: f64,
    /// Monster density (monsters per 100 floor tiles)
    pub monster_density: f64,
    /// Item density (items per 100 floor tiles)
    pub item_density: f64,
    /// Whether to use LLDM for content enhancement
    pub use_lldm: bool,
    /// LLDM enhancement probability (0.0 to 1.0)
    pub lldm_enhancement_chance: f64,
}

impl GenerationConfig {
    /// Creates a default generation configuration.
    ///
    /// # Examples
    ///
    /// ```
    /// use thatch::GenerationConfig;
    ///
    /// let config = GenerationConfig::default();
    /// assert!(config.min_room_size >= 3);
    /// assert!(config.max_room_size >= config.min_room_size);
    /// ```
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            min_room_size: 4,
            max_room_size: 12,
            min_rooms: 6,
            max_rooms: 15,
            corridor_width: 1,
            extra_connection_chance: 0.15,
            secret_door_chance: 0.05,
            monster_density: 2.0,
            item_density: 1.5,
            use_lldm: false,
            lldm_enhancement_chance: 0.3,
        }
    }

    /// Creates a configuration for testing with smaller, simpler levels.
    pub fn for_testing(seed: u64) -> Self {
        Self {
            seed,
            min_room_size: 3,
            max_room_size: 6,
            min_rooms: 3,
            max_rooms: 6,
            corridor_width: 1,
            extra_connection_chance: 0.1,
            secret_door_chance: 0.0,
            monster_density: 1.0,
            item_density: 0.5,
            use_lldm: false,
            lldm_enhancement_chance: 0.0,
        }
    }

    /// Creates a configuration for detailed, complex levels.
    pub fn for_detailed_generation(seed: u64) -> Self {
        Self {
            seed,
            min_room_size: 6,
            max_room_size: 20,
            min_rooms: 10,
            max_rooms: 25,
            corridor_width: 1,
            extra_connection_chance: 0.25,
            secret_door_chance: 0.1,
            monster_density: 3.0,
            item_density: 2.5,
            use_lldm: true,
            lldm_enhancement_chance: 0.4,
        }
    }
}

impl Default for GenerationConfig {
    fn default() -> Self {
        Self::new(42)
    }
}

/// Represents a rectangular room in the dungeon.
///
/// Rooms are the primary structural element of generated dungeons.
/// They can be enhanced by the LLDM with special properties, descriptions,
/// or unique content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Room {
    /// Unique identifier for this room
    pub id: u32,
    /// Top-left corner of the room
    pub top_left: Position,
    /// Width of the room (including walls)
    pub width: u32,
    /// Height of the room (including walls)
    pub height: u32,
    /// Type/purpose of this room
    pub room_type: RoomType,
    /// Whether this room has been discovered by the player
    pub discovered: bool,
    /// Connections to other rooms
    pub connections: Vec<u32>,
    /// Optional name for this room (LLDM can set this)
    pub name: Option<String>,
    /// Optional description (LLDM can set this)
    pub description: Option<String>,
    /// Room-specific metadata for LLDM integration
    pub metadata: HashMap<String, String>,
}

/// Different types of rooms that can be generated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoomType {
    /// Standard room with no special properties
    Normal,
    /// Room containing treasure or valuable items
    Treasure,
    /// Room with a boss or strong enemy
    Boss,
    /// Shop or merchant room
    Shop,
    /// Puzzle or challenge room
    Puzzle,
    /// Safe rest area
    Sanctuary,
    /// Library or information room
    Library,
    /// Prison or jail cells
    Prison,
    /// Throne room or important chamber
    Throne,
    /// Secret room hidden from normal exploration
    Secret,
    /// LLDM-generated room with custom properties
    LldmGenerated { subtype: String },
}

impl Room {
    /// Creates a new room with the given parameters.
    ///
    /// # Examples
    ///
    /// ```
    /// use thatch::{Room, Position, RoomType};
    ///
    /// let room = Room::new(1, Position::new(5, 5), 10, 8, RoomType::Normal);
    /// assert_eq!(room.id, 1);
    /// assert_eq!(room.width, 10);
    /// assert_eq!(room.height, 8);
    /// ```
    pub fn new(id: u32, top_left: Position, width: u32, height: u32, room_type: RoomType) -> Self {
        Self {
            id,
            top_left,
            width,
            height,
            room_type,
            discovered: false,
            connections: Vec::new(),
            name: None,
            description: None,
            metadata: HashMap::new(),
        }
    }

    /// Gets the bottom-right corner of the room.
    pub fn bottom_right(&self) -> Position {
        Position::new(
            self.top_left.x + self.width as i32 - 1,
            self.top_left.y + self.height as i32 - 1,
        )
    }

    /// Gets the center position of the room.
    pub fn center(&self) -> Position {
        Position::new(
            self.top_left.x + self.width as i32 / 2,
            self.top_left.y + self.height as i32 / 2,
        )
    }

    /// Gets the area of the room in tiles.
    pub fn area(&self) -> u32 {
        self.width * self.height
    }

    /// Gets the inner area (excluding walls) of the room.
    pub fn inner_area(&self) -> u32 {
        if self.width >= 2 && self.height >= 2 {
            (self.width - 2) * (self.height - 2)
        } else {
            0
        }
    }

    /// Checks if a position is inside this room.
    ///
    /// # Examples
    ///
    /// ```
    /// use thatch::{Room, Position, RoomType};
    ///
    /// let room = Room::new(1, Position::new(5, 5), 10, 8, RoomType::Normal);
    /// assert!(room.contains(Position::new(7, 7)));
    /// assert!(!room.contains(Position::new(20, 20)));
    /// ```
    pub fn contains(&self, pos: Position) -> bool {
        pos.x >= self.top_left.x
            && pos.y >= self.top_left.y
            && pos.x < self.top_left.x + self.width as i32
            && pos.y < self.top_left.y + self.height as i32
    }

    /// Checks if a position is on the border of this room.
    pub fn is_border(&self, pos: Position) -> bool {
        if !self.contains(pos) {
            return false;
        }

        pos.x == self.top_left.x
            || pos.y == self.top_left.y
            || pos.x == self.top_left.x + self.width as i32 - 1
            || pos.y == self.top_left.y + self.height as i32 - 1
    }

    /// Checks if this room overlaps with another room.
    pub fn overlaps(&self, other: &Room) -> bool {
        !(self.top_left.x >= other.top_left.x + other.width as i32
            || other.top_left.x >= self.top_left.x + self.width as i32
            || self.top_left.y >= other.top_left.y + other.height as i32
            || other.top_left.y >= self.top_left.y + self.height as i32)
    }

    /// Gets all floor positions within this room.
    pub fn floor_positions(&self) -> Vec<Position> {
        let mut positions = Vec::new();

        for y in (self.top_left.y + 1)..(self.top_left.y + self.height as i32 - 1) {
            for x in (self.top_left.x + 1)..(self.top_left.x + self.width as i32 - 1) {
                positions.push(Position::new(x, y));
            }
        }

        positions
    }

    /// Gets all wall positions of this room.
    pub fn wall_positions(&self) -> Vec<Position> {
        let mut positions = Vec::new();

        // Top and bottom walls
        for x in self.top_left.x..(self.top_left.x + self.width as i32) {
            positions.push(Position::new(x, self.top_left.y));
            positions.push(Position::new(x, self.top_left.y + self.height as i32 - 1));
        }

        // Left and right walls (excluding corners already added)
        for y in (self.top_left.y + 1)..(self.top_left.y + self.height as i32 - 1) {
            positions.push(Position::new(self.top_left.x, y));
            positions.push(Position::new(self.top_left.x + self.width as i32 - 1, y));
        }

        positions
    }

    /// Gets all positions within this room (both floor and walls).
    pub fn all_positions(&self) -> Vec<Position> {
        let mut positions = Vec::new();

        for y in self.top_left.y..(self.top_left.y + self.height as i32) {
            for x in self.top_left.x..(self.top_left.x + self.width as i32) {
                positions.push(Position::new(x, y));
            }
        }

        positions
    }

    /// Adds a connection to another room.
    pub fn add_connection(&mut self, room_id: u32) {
        if !self.connections.contains(&room_id) {
            self.connections.push(room_id);
        }
    }

    /// Removes a connection to another room.
    pub fn remove_connection(&mut self, room_id: u32) {
        self.connections.retain(|&id| id != room_id);
    }

    /// Sets metadata for this room (useful for LLDM integration).
    pub fn set_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }

    /// Gets metadata value by key.
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }
}

/// Trait for procedural generators.
///
/// All generation systems in Thatch implement this trait, allowing for
/// consistent interfaces and easy extension with LLDM integration.
pub trait Generator<T> {
    /// Generates content using the provided configuration and random number generator.
    fn generate(&self, config: &GenerationConfig, rng: &mut StdRng) -> ThatchResult<T>;

    /// Validates that the generated content meets requirements.
    fn validate(&self, content: &T, config: &GenerationConfig) -> ThatchResult<()>;

    /// Gets the generator type name for logging and debugging.
    fn generator_type(&self) -> &'static str;

    /// Applies LLDM enhancements to generated content (if enabled).
    fn apply_lldm_enhancements(
        &self,
        content: &mut T,
        config: &GenerationConfig,
        rng: &mut StdRng,
    ) -> ThatchResult<()> {
        // Default implementation does nothing
        let _ = (content, config, rng);
        Ok(())
    }
}

/// Utility functions for generation algorithms.
pub mod utils {
    use super::*;

    /// Creates a seeded random number generator from the config.
    pub fn create_rng(config: &GenerationConfig) -> StdRng {
        StdRng::seed_from_u64(config.seed)
    }

    /// Checks if two rooms are adjacent (for corridor placement).
    pub fn rooms_are_adjacent(room1: &Room, room2: &Room, max_distance: u32) -> bool {
        let center1 = room1.center();
        let center2 = room2.center();
        center1.manhattan_distance(center2) <= max_distance
    }

    /// Finds the best connection point between two rooms.
    pub fn find_connection_point(room1: &Room, room2: &Room) -> (Position, Position) {
        let center1 = room1.center();
        let center2 = room2.center();

        // Simple implementation: use room centers
        // In a full implementation, this would find optimal wall positions
        (center1, center2)
    }

    /// Applies smoothing to generated rooms to make them more natural.
    pub fn smooth_room_layout(rooms: &mut [Room], rng: &mut StdRng) {
        // Apply random variations to room shapes
        for room in rooms.iter_mut() {
            if rng.gen_bool(0.3) {
                // Randomly adjust room dimensions slightly
                let width_adjust = rng.gen_range(-1..=1);
                let height_adjust = rng.gen_range(-1..=1);

                room.width = ((room.width as i32 + width_adjust).max(3)) as u32;
                room.height = ((room.height as i32 + height_adjust).max(3)) as u32;
            }
        }
    }

    /// Validates that a level meets basic requirements.
    pub fn validate_level(level: &Level) -> ThatchResult<()> {
        // Check that the level has reachable areas
        let floor_count = level
            .tiles
            .iter()
            .flat_map(|row| row.iter())
            .filter(|tile| tile.tile_type == TileType::Floor)
            .count();

        if floor_count == 0 {
            return Err(ThatchError::GenerationFailed(
                "Level has no floor tiles".to_string(),
            ));
        }

        // Additional validation can be added here
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_generation_config_creation() {
        let config = GenerationConfig::new(12345);
        assert_eq!(config.seed, 12345);
        assert!(config.min_room_size >= 3);
        assert!(config.max_room_size >= config.min_room_size);
        assert!(config.min_rooms <= config.max_rooms);
    }

    #[test]
    fn test_room_creation() {
        let room = Room::new(1, Position::new(5, 5), 10, 8, RoomType::Normal);
        assert_eq!(room.id, 1);
        assert_eq!(room.top_left, Position::new(5, 5));
        assert_eq!(room.width, 10);
        assert_eq!(room.height, 8);
        assert_eq!(room.area(), 80);
        assert_eq!(room.inner_area(), 48); // (10-2) * (8-2)
    }

    #[test]
    fn test_room_geometry() {
        let room = Room::new(1, Position::new(5, 5), 10, 8, RoomType::Normal);

        assert_eq!(room.bottom_right(), Position::new(14, 12));
        assert_eq!(room.center(), Position::new(10, 9));

        assert!(room.contains(Position::new(7, 7)));
        assert!(room.contains(Position::new(5, 5))); // Top-left corner
        assert!(room.contains(Position::new(14, 12))); // Bottom-right corner
        assert!(!room.contains(Position::new(4, 5))); // Outside left
        assert!(!room.contains(Position::new(15, 12))); // Outside right

        assert!(room.is_border(Position::new(5, 5))); // Top-left corner
        assert!(room.is_border(Position::new(10, 5))); // Top edge
        assert!(!room.is_border(Position::new(7, 7))); // Interior
    }

    #[test]
    fn test_room_overlap() {
        let room1 = Room::new(1, Position::new(5, 5), 10, 8, RoomType::Normal);
        let room2 = Room::new(2, Position::new(10, 8), 6, 6, RoomType::Normal); // Overlaps
        let room3 = Room::new(3, Position::new(20, 20), 5, 5, RoomType::Normal); // No overlap

        assert!(room1.overlaps(&room2));
        assert!(room2.overlaps(&room1));
        assert!(!room1.overlaps(&room3));
        assert!(!room3.overlaps(&room1));
    }

    #[test]
    fn test_room_positions() {
        let room = Room::new(1, Position::new(5, 5), 4, 4, RoomType::Normal);

        let floor_positions = room.floor_positions();
        let wall_positions = room.wall_positions();

        // 4x4 room should have 2x2 = 4 floor tiles
        assert_eq!(floor_positions.len(), 4);

        // Should have 4*4 - 2*2 = 12 wall tiles
        assert_eq!(wall_positions.len(), 12);

        // Check that floor and wall positions don't overlap
        let floor_set: HashSet<_> = floor_positions.into_iter().collect();
        let wall_set: HashSet<_> = wall_positions.into_iter().collect();
        assert!(floor_set.is_disjoint(&wall_set));
    }

    #[test]
    fn test_room_connections() {
        let mut room = Room::new(1, Position::new(5, 5), 10, 8, RoomType::Normal);

        assert!(room.connections.is_empty());

        room.add_connection(2);
        room.add_connection(3);
        assert_eq!(room.connections.len(), 2);
        assert!(room.connections.contains(&2));
        assert!(room.connections.contains(&3));

        // Adding same connection should not duplicate
        room.add_connection(2);
        assert_eq!(room.connections.len(), 2);

        room.remove_connection(2);
        assert_eq!(room.connections.len(), 1);
        assert!(!room.connections.contains(&2));
        assert!(room.connections.contains(&3));
    }

    #[test]
    fn test_room_metadata() {
        let mut room = Room::new(1, Position::new(5, 5), 10, 8, RoomType::Normal);

        assert!(room.get_metadata("description").is_none());

        room.set_metadata("description".to_string(), "A dark chamber".to_string());
        assert_eq!(
            room.get_metadata("description"),
            Some(&"A dark chamber".to_string())
        );

        room.set_metadata("loot_level".to_string(), "high".to_string());
        assert_eq!(room.get_metadata("loot_level"), Some(&"high".to_string()));
    }

    #[test]
    fn test_utils_rng_creation() {
        let config = GenerationConfig::new(12345);
        let _rng = utils::create_rng(&config);
        // RNG creation should not panic
    }

    #[test]
    fn test_utils_room_adjacency() {
        let room1 = Room::new(1, Position::new(5, 5), 5, 5, RoomType::Normal);
        let room2 = Room::new(2, Position::new(12, 5), 5, 5, RoomType::Normal); // Close
        let room3 = Room::new(3, Position::new(50, 50), 5, 5, RoomType::Normal); // Far

        assert!(utils::rooms_are_adjacent(&room1, &room2, 20));
        assert!(!utils::rooms_are_adjacent(&room1, &room3, 20));
    }
}
