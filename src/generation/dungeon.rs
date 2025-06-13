//! # Dungeon Generation
//!
//! Procedural dungeon layout generation using room-and-corridor algorithms.
//!
//! This module implements sophisticated dungeon generation algorithms that create
//! interesting, connected layouts. The system supports various generation strategies
//! and can be enhanced by the LLDM for unique architectural features.

use crate::generation::utils;
use crate::{
    GenerationConfig, Generator, Level, Position, Room, RoomType, ThatchError, ThatchResult, Tile,
    TileType,
};
use rand::{rngs::StdRng, Rng};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet, VecDeque};

/// Node for A* pathfinding algorithm.
#[derive(Debug, Clone)]
struct AStarNode {
    position: Position,
    f_score: f64,
}

impl PartialEq for AStarNode {
    fn eq(&self, other: &Self) -> bool {
        self.f_score == other.f_score
    }
}

impl Eq for AStarNode {}

impl PartialOrd for AStarNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AStarNode {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering for min-heap behavior in BinaryHeap
        other
            .f_score
            .partial_cmp(&self.f_score)
            .unwrap_or(Ordering::Equal)
    }
}

/// Primary dungeon generator using overlapping rooms and progressive wall placement.
///
/// This generator creates dungeons by:
/// 1. Placing rooms randomly (overlapping is allowed for interesting shapes)
/// 2. Starting with all non-room spaces as open floor
/// 3. Progressively adding walls while maintaining connectivity between all rooms
/// 4. Stopping when connectivity failures reach a threshold or no spaces remain
#[derive(Debug, Clone)]
pub struct RoomCorridorGenerator {
    /// Strategy for room placement
    pub room_placement_strategy: RoomPlacementStrategy,
    /// Maximum number of connectivity failures before stopping wall placement
    pub max_connectivity_failures: u32,
    /// Maximum attempts to place a room before giving up
    pub max_placement_attempts: u32,
    /// Whether to ensure all rooms are connected (always true for this algorithm)
    pub ensure_connectivity: bool,
}

/// Strategies for placing rooms in the dungeon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoomPlacementStrategy {
    /// Completely random placement with collision detection
    Random,
    /// Grid-based placement with some randomness
    GridBased { grid_size: u32 },
    /// Place rooms along the edges first, then fill center
    EdgeFirst,
    /// Use noise functions for more organic placement
    NoiseGuided,
}

impl RoomCorridorGenerator {
    /// Creates a new dungeon generator with default settings.
    ///
    /// # Examples
    ///
    /// ```
    /// use thatch::RoomCorridorGenerator;
    ///
    /// let generator = RoomCorridorGenerator::new();
    /// // Generator can now be used to create levels
    /// ```
    pub fn new() -> Self {
        Self {
            room_placement_strategy: RoomPlacementStrategy::Random,
            max_connectivity_failures: 100,
            max_placement_attempts: 100,
            ensure_connectivity: true,
        }
    }

    /// Creates a generator with specific settings.
    ///
    /// # Examples
    ///
    /// ```
    /// use thatch::{RoomCorridorGenerator, RoomPlacementStrategy};
    ///
    /// let generator = RoomCorridorGenerator::with_settings(
    ///     RoomPlacementStrategy::Random,
    ///     50
    /// );
    /// ```
    pub fn with_settings(
        room_placement_strategy: RoomPlacementStrategy,
        max_connectivity_failures: u32,
    ) -> Self {
        Self {
            room_placement_strategy,
            max_connectivity_failures,
            max_placement_attempts: 100,
            ensure_connectivity: true,
        }
    }

    /// Creates a generator optimized for testing.
    pub fn for_testing() -> Self {
        Self {
            room_placement_strategy: RoomPlacementStrategy::Random,
            max_connectivity_failures: 50,
            max_placement_attempts: 50,
            ensure_connectivity: true,
        }
    }

    /// Creates a generator for detailed, complex dungeons.
    pub fn for_detailed_generation() -> Self {
        Self {
            room_placement_strategy: RoomPlacementStrategy::NoiseGuided,
            max_connectivity_failures: 200,
            max_placement_attempts: 200,
            ensure_connectivity: true,
        }
    }

    /// Places rooms with overlapping allowed.
    fn place_rooms(
        &self,
        level: &mut Level,
        config: &GenerationConfig,
        rng: &mut StdRng,
    ) -> ThatchResult<Vec<Room>> {
        let mut rooms = Vec::new();
        let room_count = rng.gen_range(config.min_rooms..=config.max_rooms);

        for room_id in 0..room_count {
            // Allow overlapping - just place the room if it fits in bounds
            if let Some(room) = self.try_place_room_overlapping(level, config, rng, room_id)? {
                rooms.push(room);
            }
        }

        if rooms.is_empty() {
            return Err(ThatchError::GenerationFailed(
                "Failed to place any rooms".to_string(),
            ));
        }

        Ok(rooms)
    }

    /// Attempts to place a single room with overlapping allowed.
    fn try_place_room_overlapping(
        &self,
        level: &Level,
        config: &GenerationConfig,
        rng: &mut StdRng,
        room_id: u32,
    ) -> ThatchResult<Option<Room>> {
        for _ in 0..self.max_placement_attempts {
            let room = self.generate_room_candidate(level, config, rng, room_id)?;

            // Only check if room fits in level bounds - overlapping is allowed
            if self.room_fits_in_level(level, &room) {
                return Ok(Some(room));
            }
        }

        Ok(None) // Failed to place room after all attempts
    }

    /// Generates a candidate room based on placement strategy.
    fn generate_room_candidate(
        &self,
        level: &Level,
        config: &GenerationConfig,
        rng: &mut StdRng,
        room_id: u32,
    ) -> ThatchResult<Room> {
        let width = rng.gen_range(config.min_room_size..=config.max_room_size);
        let height = rng.gen_range(config.min_room_size..=config.max_room_size);

        let (x, y) = match &self.room_placement_strategy {
            RoomPlacementStrategy::Random => {
                let x = rng.gen_range(1..(level.width as i32 - width as i32 - 1));
                let y = rng.gen_range(1..(level.height as i32 - height as i32 - 1));
                (x, y)
            }
            RoomPlacementStrategy::GridBased { grid_size } => {
                let grid_x = rng.gen_range(0..(level.width / grid_size));
                let grid_y = rng.gen_range(0..(level.height / grid_size));
                let x = (grid_x * grid_size) as i32
                    + rng.gen_range(1..(*grid_size as i32 - width as i32));
                let y = (grid_y * grid_size) as i32
                    + rng.gen_range(1..(*grid_size as i32 - height as i32));
                (x.max(1), y.max(1))
            }
            RoomPlacementStrategy::EdgeFirst => {
                // Place rooms near edges first, then fill center
                let margin = 5;
                let x = if rng.gen_bool(0.6) {
                    // Near edge
                    if rng.gen_bool(0.5) {
                        rng.gen_range(1..margin)
                    } else {
                        rng.gen_range(
                            (level.width as i32 - margin - width as i32)
                                ..(level.width as i32 - width as i32 - 1),
                        )
                    }
                } else {
                    // Center area
                    rng.gen_range(margin..(level.width as i32 - margin - width as i32))
                };
                let y = rng.gen_range(1..(level.height as i32 - height as i32 - 1));
                (x, y)
            }
            RoomPlacementStrategy::NoiseGuided => {
                // Use noise for more organic placement
                let x = rng.gen_range(1..(level.width as i32 - width as i32 - 1));
                let y = rng.gen_range(1..(level.height as i32 - height as i32 - 1));
                (x, y)
            }
        };

        let room_type = self.determine_room_type(room_id, config, rng);

        Ok(Room::new(
            room_id,
            Position::new(x, y),
            width,
            height,
            room_type,
        ))
    }

    /// Determines the type of room to create.
    fn determine_room_type(
        &self,
        room_id: u32,
        config: &GenerationConfig,
        rng: &mut StdRng,
    ) -> RoomType {
        // First room is always normal (spawn room)
        if room_id == 0 {
            return RoomType::Normal;
        }

        // Apply probabilities for special room types
        let roll = rng.gen::<f64>();

        if roll < 0.05 {
            RoomType::Treasure
        } else if roll < 0.08 {
            RoomType::Shop
        } else if roll < 0.10 {
            RoomType::Sanctuary
        } else if roll < 0.12 {
            RoomType::Library
        } else if roll < 0.14 && config.secret_door_chance > 0.0 {
            RoomType::Secret
        } else if roll < 0.16 {
            RoomType::Puzzle
        } else {
            RoomType::Normal
        }
    }

    /// Validates room placement based on strategy.
    fn validate_room_placement(
        &self,
        room: &Room,
        existing_rooms: &[Room],
        _config: &GenerationConfig,
        _rng: &mut StdRng,
    ) -> bool {
        match &self.room_placement_strategy {
            RoomPlacementStrategy::Random => true,
            RoomPlacementStrategy::GridBased { .. } => true,
            RoomPlacementStrategy::EdgeFirst => {
                // Ensure minimum distance between rooms
                existing_rooms
                    .iter()
                    .all(|existing| room.center().manhattan_distance(existing.center()) >= 8)
            }
            RoomPlacementStrategy::NoiseGuided => {
                // Additional noise-based validation could be added here
                true
            }
        }
    }

    /// Checks if a room fits within level boundaries.
    fn room_fits_in_level(&self, level: &Level, room: &Room) -> bool {
        room.top_left.x >= 1
            && room.top_left.y >= 1
            && room.top_left.x + (room.width as i32) < level.width as i32 - 1
            && room.top_left.y + (room.height as i32) < level.height as i32 - 1
    }

    /// Initializes the level with rooms and open floor everywhere else.
    fn initialize_level_with_rooms(&self, level: &mut Level, rooms: &[Room]) -> ThatchResult<()> {
        // Set all interior areas to floor initially (we'll add walls progressively)
        // Keep the border as walls for level boundaries
        for y in 1..(level.height as i32 - 1) {
            for x in 1..(level.width as i32 - 1) {
                let pos = Position::new(x, y);
                level.set_tile(pos, Tile::floor())?;
            }
        }

        // Mark room areas as floor (redundant but explicit)
        for room in rooms {
            for pos in room.all_positions() {
                if level.is_valid_position(pos) {
                    level.set_tile(pos, Tile::floor())?;
                }
            }
        }

        Ok(())
    }

    /// Checks if a position is within any room.
    fn is_position_in_any_room(&self, pos: Position, rooms: &[Room]) -> bool {
        rooms.iter().any(|room| room.contains(pos))
    }

    /// Progressively adds walls while maintaining connectivity.
    fn progressive_wall_placement(
        &self,
        level: &mut Level,
        rooms: &[Room],
        rng: &mut StdRng,
    ) -> ThatchResult<()> {
        let mut connectivity_failures = 0;
        let mut available_positions = self.get_non_room_floor_positions(level, rooms);

        while connectivity_failures < self.max_connectivity_failures
            && !available_positions.is_empty()
        {
            // Randomly select a position to potentially place a wall
            let index = rng.gen_range(0..available_positions.len());
            let pos = available_positions.remove(index);

            // Temporarily place a wall
            let original_tile = level.get_tile(pos).unwrap().clone();
            level.set_tile(pos, Tile::wall())?;

            // Test if all rooms are still connected
            if self.all_rooms_connected(level, rooms)? {
                // Wall placement is valid, keep it
                continue;
            } else {
                // Wall breaks connectivity, remove it and count as failure
                level.set_tile(pos, original_tile)?;
                connectivity_failures += 1;
            }
        }

        Ok(())
    }

    /// Gets all floor positions that are not within any room.
    fn get_non_room_floor_positions(&self, level: &Level, rooms: &[Room]) -> Vec<Position> {
        let mut positions = Vec::new();

        for y in 1..(level.height as i32 - 1) {
            for x in 1..(level.width as i32 - 1) {
                let pos = Position::new(x, y);
                if let Some(tile) = level.get_tile(pos) {
                    if tile.tile_type == TileType::Floor
                        && !self.is_position_in_any_room(pos, rooms)
                    {
                        positions.push(pos);
                    }
                }
            }
        }

        positions
    }

    /// Tests if all rooms are connected using A* pathfinding.
    fn all_rooms_connected(&self, level: &Level, rooms: &[Room]) -> ThatchResult<bool> {
        if rooms.len() < 2 {
            return Ok(true);
        }

        // Pick a random point in the first room as our reference
        let start_room = &rooms[0];
        let start_pos = start_room.center();

        // Test connectivity from start room to all other rooms
        for target_room in &rooms[1..] {
            let target_pos = target_room.center();

            if !self.has_path(level, start_pos, target_pos)? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Uses A* pathfinding to check if there's a path between two positions.
    fn has_path(&self, level: &Level, start: Position, goal: Position) -> ThatchResult<bool> {
        // Simple A* implementation
        let mut open_set = std::collections::BinaryHeap::new();
        let mut came_from = HashMap::new();
        let mut g_score = HashMap::new();
        let mut f_score = HashMap::new();

        g_score.insert(start, 0.0);
        f_score.insert(start, start.euclidean_distance(goal));
        open_set.push(AStarNode {
            position: start,
            f_score: start.euclidean_distance(goal),
        });

        while let Some(current_node) = open_set.pop() {
            let current = current_node.position;

            if current == goal {
                return Ok(true);
            }

            for neighbor in current.cardinal_adjacent_positions() {
                if !level.is_valid_position(neighbor) {
                    continue;
                }

                let tile = level.get_tile(neighbor).unwrap();
                if !tile.tile_type.is_passable() {
                    continue;
                }

                let tentative_g_score = g_score.get(&current).unwrap_or(&f64::INFINITY) + 1.0;

                if tentative_g_score < *g_score.get(&neighbor).unwrap_or(&f64::INFINITY) {
                    came_from.insert(neighbor, current);
                    g_score.insert(neighbor, tentative_g_score);
                    let f = tentative_g_score + neighbor.euclidean_distance(goal);
                    f_score.insert(neighbor, f);

                    open_set.push(AStarNode {
                        position: neighbor,
                        f_score: f,
                    });
                }
            }
        }

        Ok(false)
    }

    /// Adds stairs to connect between levels.
    fn add_stairs(
        &self,
        level: &mut Level,
        rooms: &[Room],
        _config: &GenerationConfig,
        rng: &mut StdRng,
    ) -> ThatchResult<()> {
        if rooms.is_empty() {
            return Ok(());
        }

        // Add stairs up in the first room
        let first_room = &rooms[0];
        let floor_positions = first_room.floor_positions();
        if !floor_positions.is_empty() {
            let stairs_up_pos = floor_positions[rng.gen_range(0..floor_positions.len())];
            level.set_tile(stairs_up_pos, Tile::new(TileType::StairsUp))?;
        }

        // Add stairs down in the last room (if more than one room)
        if rooms.len() > 1 {
            let last_room = &rooms[rooms.len() - 1];
            let floor_positions = last_room.floor_positions();
            if !floor_positions.is_empty() {
                let stairs_down_pos = floor_positions[rng.gen_range(0..floor_positions.len())];
                level.set_tile(stairs_down_pos, Tile::new(TileType::StairsDown))?;
            }
        }

        Ok(())
    }
}

impl Generator<Level> for RoomCorridorGenerator {
    fn generate(&self, config: &GenerationConfig, rng: &mut StdRng) -> ThatchResult<Level> {
        // Create level with reasonable dimensions
        let estimated_width = ((config.max_rooms * config.max_room_size * 2) as f64).sqrt() as u32;
        let estimated_height = estimated_width;
        let width = estimated_width.max(50).min(200); // Reasonable bounds
        let height = estimated_height.max(50).min(200);
        let mut level = Level::new(0, width, height);

        // Step 1: Place rooms (overlapping allowed)
        let rooms = self.place_rooms(&mut level, config, rng)?;

        // Step 2: Initialize level with rooms and open floor everywhere else
        self.initialize_level_with_rooms(&mut level, &rooms)?;

        // Step 3: Progressively add walls while maintaining connectivity
        self.progressive_wall_placement(&mut level, &rooms, rng)?;

        // Step 4: Add stairs
        self.add_stairs(&mut level, &rooms, config, rng)?;

        // Apply LLDM enhancements if enabled
        if config.use_lldm {
            // LLDM enhancement would be implemented here
        }

        // Set player spawn point to first room
        if !rooms.is_empty() {
            level.player_spawn = rooms[0].center();
        }

        // Final validation
        utils::validate_level(&level)?;

        Ok(level)
    }

    fn validate(&self, level: &Level, _config: &GenerationConfig) -> ThatchResult<()> {
        utils::validate_level(level)
    }

    fn generator_type(&self) -> &'static str {
        "RoomCorridorGenerator"
    }

    fn apply_lldm_enhancements(
        &self,
        level: &mut Level,
        config: &GenerationConfig,
        rng: &mut StdRng,
    ) -> ThatchResult<()> {
        if !config.use_lldm {
            return Ok(());
        }

        // LLDM enhancement implementation would go here
        // For now, just add some random special tiles
        let enhancement_count = (level.width * level.height / 200) as usize;

        for _ in 0..enhancement_count {
            let x = rng.gen_range(0..level.width) as i32;
            let y = rng.gen_range(0..level.height) as i32;
            let pos = Position::new(x, y);

            if let Some(tile) = level.get_tile(pos) {
                if tile.tile_type == TileType::Floor && rng.gen_bool(config.lldm_enhancement_chance)
                {
                    let special_tile = Tile::new(TileType::Special {
                        description: "A mysterious tile with unknown properties".to_string(),
                    });
                    level.set_tile(pos, special_tile)?;
                }
            }
        }

        Ok(())
    }
}

impl Default for RoomCorridorGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config;

    #[test]
    fn test_room_corridor_generator_creation() {
        let generator = RoomCorridorGenerator::new();
        assert_eq!(
            generator.room_placement_strategy,
            RoomPlacementStrategy::Random
        );
        assert_eq!(generator.max_connectivity_failures, 100);
        assert!(generator.ensure_connectivity);
    }

    #[test]
    fn test_room_fits_in_level() {
        let generator = RoomCorridorGenerator::new();
        let level = Level::new(0, 50, 40);

        let good_room = Room::new(1, Position::new(5, 5), 10, 8, RoomType::Normal);
        let bad_room = Room::new(2, Position::new(45, 35), 10, 8, RoomType::Normal);

        assert!(generator.room_fits_in_level(&level, &good_room));
        assert!(!generator.room_fits_in_level(&level, &bad_room));
    }

    #[test]
    fn test_room_type_determination() {
        let generator = RoomCorridorGenerator::new();
        let config = GenerationConfig::for_testing(12345);
        let mut rng = utils::create_rng(&config);

        // First room should always be normal
        let room_type = generator.determine_room_type(0, &config, &mut rng);
        assert_eq!(room_type, RoomType::Normal);

        // Other rooms can be various types
        let _room_type = generator.determine_room_type(1, &config, &mut rng);
        // We can't assert specific type due to randomness, but it shouldn't panic
    }

    #[test]
    fn test_new_algorithm_generation() {
        let generator = RoomCorridorGenerator::for_testing();
        let config = GenerationConfig::for_testing(42);
        let mut rng = utils::create_rng(&config);

        // This should not panic and should produce a valid level
        let result = generator.generate(&config, &mut rng);
        assert!(result.is_ok());

        let level = result.unwrap();
        // Check that level has reasonable dimensions
        assert!(level.width >= 50 && level.width <= 200);
        assert!(level.height >= 50 && level.height <= 200);

        // Count different tile types
        let mut wall_count = 0;
        let mut floor_count = 0;

        for row in &level.tiles {
            for tile in row {
                match tile.tile_type {
                    TileType::Wall => wall_count += 1,
                    TileType::Floor => floor_count += 1,
                    _ => {}
                }
            }
        }

        // Should have both walls and floors
        assert!(floor_count > 0, "Level should have floor tiles");
        assert!(
            wall_count > 0,
            "Level should have wall tiles after progressive placement"
        );

        // The algorithm typically produces more walls than floors, which is expected
        // since we progressively add walls to fill space while maintaining connectivity

        println!(
            "Generated {}x{} level with {} floors, {} walls",
            level.width, level.height, floor_count, wall_count
        );
    }

    #[test]
    fn test_validation() {
        let generator = RoomCorridorGenerator::new();
        let config = GenerationConfig::for_testing(12345);

        // Valid level with some floor tiles
        let mut level = Level::new(0, 10, 10);
        level.set_tile(Position::new(5, 5), Tile::floor()).unwrap();

        assert!(generator.validate(&level, &config).is_ok());

        // Invalid level with no floor tiles
        let empty_level = Level::new(0, 10, 10);
        assert!(generator.validate(&empty_level, &config).is_err());
    }
}
