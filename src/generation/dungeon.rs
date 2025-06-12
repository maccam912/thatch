//! # Dungeon Generation
//! 
//! Procedural dungeon layout generation using room-and-corridor algorithms.
//! 
//! This module implements sophisticated dungeon generation algorithms that create
//! interesting, connected layouts. The system supports various generation strategies
//! and can be enhanced by the LLDM for unique architectural features.

use crate::{
    ThatchResult, ThatchError, Level, Position, TileType, Tile, 
    GenerationConfig, Generator, Room, RoomType, utils
};
use rand::{Rng, rngs::StdRng};
use std::collections::{HashMap, HashSet, VecDeque};

/// Primary dungeon generator using room-and-corridor algorithm.
/// 
/// This generator creates dungeons by:
/// 1. Placing rooms randomly with collision detection
/// 2. Connecting rooms with corridors using pathfinding
/// 3. Adding doors, stairs, and special features
/// 4. Optionally enhancing with LLDM-generated content
#[derive(Debug, Clone)]
pub struct RoomCorridorGenerator {
    /// Strategy for room placement
    pub room_placement_strategy: RoomPlacementStrategy,
    /// Strategy for corridor generation
    pub corridor_strategy: CorridorStrategy,
    /// Maximum attempts to place a room before giving up
    pub max_placement_attempts: u32,
    /// Whether to ensure all rooms are connected
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

/// Strategies for connecting rooms with corridors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CorridorStrategy {
    /// Simple L-shaped corridors
    LShaped,
    /// Straight corridors when possible
    Straight,
    /// Winding corridors for more interesting layouts
    Winding,
    /// Minimum spanning tree for optimal connectivity
    MinimumSpanningTree,
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
            corridor_strategy: CorridorStrategy::LShaped,
            max_placement_attempts: 100,
            ensure_connectivity: true,
        }
    }
    
    /// Creates a generator optimized for testing.
    pub fn for_testing() -> Self {
        Self {
            room_placement_strategy: RoomPlacementStrategy::GridBased { grid_size: 10 },
            corridor_strategy: CorridorStrategy::Straight,
            max_placement_attempts: 50,
            ensure_connectivity: true,
        }
    }
    
    /// Creates a generator for detailed, complex dungeons.
    pub fn for_detailed_generation() -> Self {
        Self {
            room_placement_strategy: RoomPlacementStrategy::NoiseGuided,
            corridor_strategy: CorridorStrategy::MinimumSpanningTree,
            max_placement_attempts: 200,
            ensure_connectivity: true,
        }
    }
    
    /// Places rooms according to the configured strategy.
    fn place_rooms(
        &self,
        level: &mut Level,
        config: &GenerationConfig,
        rng: &mut StdRng,
    ) -> ThatchResult<Vec<Room>> {
        let mut rooms = Vec::new();
        let room_count = rng.gen_range(config.min_rooms..=config.max_rooms);
        
        for room_id in 0..room_count {
            if let Some(room) = self.try_place_room(level, config, rng, room_id, &rooms)? {
                self.carve_room(level, &room)?;
                rooms.push(room);
            }
        }
        
        if rooms.is_empty() {
            return Err(ThatchError::GenerationFailed(
                "Failed to place any rooms".to_string()
            ));
        }
        
        Ok(rooms)
    }
    
    /// Attempts to place a single room.
    fn try_place_room(
        &self,
        level: &Level,
        config: &GenerationConfig,
        rng: &mut StdRng,
        room_id: u32,
        existing_rooms: &[Room],
    ) -> ThatchResult<Option<Room>> {
        for _ in 0..self.max_placement_attempts {
            let room = self.generate_room_candidate(level, config, rng, room_id)?;
            
            // Check if room fits in level bounds
            if !self.room_fits_in_level(level, &room) {
                continue;
            }
            
            // Check for overlap with existing rooms
            if existing_rooms.iter().any(|existing| room.overlaps(existing)) {
                continue;
            }
            
            // Additional validation based on placement strategy
            if self.validate_room_placement(&room, existing_rooms, config, rng) {
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
                let x = (grid_x * grid_size) as i32 + rng.gen_range(1..(*grid_size as i32 - width as i32));
                let y = (grid_y * grid_size) as i32 + rng.gen_range(1..(*grid_size as i32 - height as i32));
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
                        rng.gen_range((level.width as i32 - margin - width as i32)..(level.width as i32 - width as i32 - 1))
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
        
        Ok(Room::new(room_id, Position::new(x, y), width, height, room_type))
    }
    
    /// Determines the type of room to create.
    fn determine_room_type(&self, room_id: u32, config: &GenerationConfig, rng: &mut StdRng) -> RoomType {
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
                existing_rooms.iter().all(|existing| {
                    room.center().manhattan_distance(existing.center()) >= 8
                })
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
            && room.top_left.x + room.width as i32 < level.width as i32 - 1
            && room.top_left.y + room.height as i32 < level.height as i32 - 1
    }
    
    /// Carves out a room in the level by setting tiles to floor.
    fn carve_room(&self, level: &mut Level, room: &Room) -> ThatchResult<()> {
        // Carve out floor tiles
        for pos in room.floor_positions() {
            level.set_tile(pos, Tile::floor())?;
        }
        
        // Leave walls as walls (they're already walls from initialization)
        Ok(())
    }
    
    /// Connects rooms with corridors.
    fn connect_rooms(
        &self,
        level: &mut Level,
        rooms: &mut Vec<Room>,
        config: &GenerationConfig,
        rng: &mut StdRng,
    ) -> ThatchResult<()> {
        if rooms.len() < 2 {
            return Ok(()); // Nothing to connect
        }
        
        match &self.corridor_strategy {
            CorridorStrategy::LShaped => self.connect_with_l_corridors(level, rooms, config, rng),
            CorridorStrategy::Straight => self.connect_with_straight_corridors(level, rooms, config, rng),
            CorridorStrategy::Winding => self.connect_with_winding_corridors(level, rooms, config, rng),
            CorridorStrategy::MinimumSpanningTree => self.connect_with_mst(level, rooms, config, rng),
        }
    }
    
    /// Connects rooms using L-shaped corridors.
    fn connect_with_l_corridors(
        &self,
        level: &mut Level,
        rooms: &mut Vec<Room>,
        config: &GenerationConfig,
        rng: &mut StdRng,
    ) -> ThatchResult<()> {
        // Connect each room to the next one
        for i in 0..(rooms.len() - 1) {
            let room1_center = rooms[i].center();
            let room2_center = rooms[i + 1].center();
            
            self.carve_l_corridor(level, room1_center, room2_center, config)?;
            
            // Add connection tracking
            rooms[i].add_connection(rooms[i + 1].id);
            rooms[i + 1].add_connection(rooms[i].id);
        }
        
        // Add some extra connections for variety
        let extra_connections = (rooms.len() as f64 * config.extra_connection_chance) as usize;
        for _ in 0..extra_connections {
            let room1_idx = rng.gen_range(0..rooms.len());
            let room2_idx = rng.gen_range(0..rooms.len());
            
            if room1_idx != room2_idx {
                let room1_center = rooms[room1_idx].center();
                let room2_center = rooms[room2_idx].center();
                
                self.carve_l_corridor(level, room1_center, room2_center, config)?;
                
                rooms[room1_idx].add_connection(rooms[room2_idx].id);
                rooms[room2_idx].add_connection(rooms[room1_idx].id);
            }
        }
        
        Ok(())
    }
    
    /// Carves an L-shaped corridor between two points.
    fn carve_l_corridor(
        &self,
        level: &mut Level,
        start: Position,
        end: Position,
        _config: &GenerationConfig,
    ) -> ThatchResult<()> {
        // Choose whether to go horizontal first or vertical first
        let horizontal_first = start.x != end.x;
        
        if horizontal_first {
            // Horizontal segment
            let min_x = start.x.min(end.x);
            let max_x = start.x.max(end.x);
            for x in min_x..=max_x {
                let pos = Position::new(x, start.y);
                if level.is_valid_position(pos) {
                    level.set_tile(pos, Tile::floor())?;
                }
            }
            
            // Vertical segment
            let min_y = start.y.min(end.y);
            let max_y = start.y.max(end.y);
            for y in min_y..=max_y {
                let pos = Position::new(end.x, y);
                if level.is_valid_position(pos) {
                    level.set_tile(pos, Tile::floor())?;
                }
            }
        } else {
            // Vertical segment first
            let min_y = start.y.min(end.y);
            let max_y = start.y.max(end.y);
            for y in min_y..=max_y {
                let pos = Position::new(start.x, y);
                if level.is_valid_position(pos) {
                    level.set_tile(pos, Tile::floor())?;
                }
            }
            
            // Horizontal segment
            let min_x = start.x.min(end.x);
            let max_x = start.x.max(end.x);
            for x in min_x..=max_x {
                let pos = Position::new(x, end.y);
                if level.is_valid_position(pos) {
                    level.set_tile(pos, Tile::floor())?;
                }
            }
        }
        
        Ok(())
    }
    
    /// Connects rooms with straight corridors (placeholder).
    fn connect_with_straight_corridors(
        &self,
        level: &mut Level,
        rooms: &mut Vec<Room>,
        config: &GenerationConfig,
        rng: &mut StdRng,
    ) -> ThatchResult<()> {
        // For now, use L-shaped as fallback
        self.connect_with_l_corridors(level, rooms, config, rng)
    }
    
    /// Connects rooms with winding corridors (placeholder).
    fn connect_with_winding_corridors(
        &self,
        level: &mut Level,
        rooms: &mut Vec<Room>,
        config: &GenerationConfig,
        rng: &mut StdRng,
    ) -> ThatchResult<()> {
        // For now, use L-shaped as fallback
        self.connect_with_l_corridors(level, rooms, config, rng)
    }
    
    /// Connects rooms using minimum spanning tree (placeholder).
    fn connect_with_mst(
        &self,
        level: &mut Level,
        rooms: &mut Vec<Room>,
        config: &GenerationConfig,
        rng: &mut StdRng,
    ) -> ThatchResult<()> {
        // For now, use L-shaped as fallback
        self.connect_with_l_corridors(level, rooms, config, rng)
    }
    
    /// Adds doors between rooms and corridors.
    fn add_doors(
        &self,
        level: &mut Level,
        rooms: &[Room],
        _config: &GenerationConfig,
        rng: &mut StdRng,
    ) -> ThatchResult<()> {
        for room in rooms {
            // Find potential door positions (room walls adjacent to corridors)
            for wall_pos in room.wall_positions() {
                if !level.is_valid_position(wall_pos) {
                    continue;
                }
                
                // Check if this wall position has floor on the other side
                let adjacent_positions = wall_pos.cardinal_adjacent_positions();
                let has_adjacent_floor = adjacent_positions.iter().any(|&pos| {
                    level.get_tile(pos)
                        .map(|tile| tile.tile_type == TileType::Floor)
                        .unwrap_or(false)
                });
                
                if has_adjacent_floor && rng.gen_bool(0.3) {
                    // Place a door
                    let door_tile = Tile::new(TileType::Door { is_open: false });
                    level.set_tile(wall_pos, door_tile)?;
                }
            }
        }
        
        Ok(())
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
    
    /// Validates that all rooms are reachable from each other.
    fn validate_connectivity(&self, level: &Level, rooms: &[Room]) -> ThatchResult<()> {
        if !self.ensure_connectivity || rooms.is_empty() {
            return Ok(());
        }
        
        // Use flood fill to check if all room floor tiles are connected
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        
        // Start from the first room's center
        let start_pos = rooms[0].center();
        queue.push_back(start_pos);
        visited.insert(start_pos);
        
        while let Some(pos) = queue.pop_front() {
            for adjacent_pos in pos.cardinal_adjacent_positions() {
                if visited.contains(&adjacent_pos) {
                    continue;
                }
                
                if let Some(tile) = level.get_tile(adjacent_pos) {
                    if tile.tile_type.is_passable() {
                        visited.insert(adjacent_pos);
                        queue.push_back(adjacent_pos);
                    }
                }
            }
        }
        
        // Check if all room floor tiles are reachable
        for room in rooms {
            for floor_pos in room.floor_positions() {
                if !visited.contains(&floor_pos) {
                    return Err(ThatchError::GenerationFailed(
                        format!("Room {} is not connected to other rooms", room.id)
                    ));
                }
            }
        }
        
        Ok(())
    }
}

impl Generator<Level> for RoomCorridorGenerator {
    fn generate(&self, config: &GenerationConfig, rng: &mut StdRng) -> ThatchResult<Level> {
        // Create empty level (all walls)
        let mut level = Level::new(0, config.seed as u32, config.seed as u32);
        
        // Place rooms
        let mut rooms = self.place_rooms(&mut level, config, rng)?;
        
        // Connect rooms with corridors
        self.connect_rooms(&mut level, &mut rooms, config, rng)?;
        
        // Add doors
        self.add_doors(&mut level, &rooms, config, rng)?;
        
        // Add stairs
        self.add_stairs(&mut level, &rooms, config, rng)?;
        
        // Validate connectivity
        self.validate_connectivity(&level, &rooms)?;
        
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
                if tile.tile_type == TileType::Floor && rng.gen_bool(config.lldm_enhancement_chance) {
                    let special_tile = Tile::new(TileType::Special { 
                        description: "A mysterious tile with unknown properties".to_string() 
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
        assert_eq!(generator.room_placement_strategy, RoomPlacementStrategy::Random);
        assert_eq!(generator.corridor_strategy, CorridorStrategy::LShaped);
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
    fn test_generation_with_small_level() {
        let generator = RoomCorridorGenerator::for_testing();
        let config = GenerationConfig::for_testing(12345);
        let mut rng = utils::create_rng(&config);
        
        // This should not panic and should produce a valid level
        let result = generator.generate(&config, &mut rng);
        assert!(result.is_ok());
        
        let level = result.unwrap();
        assert_eq!(level.width, config.seed as u32);
        assert_eq!(level.height, config.seed as u32);
        
        // Should have some floor tiles
        let floor_count = level.tiles.iter()
            .flat_map(|row| row.iter())
            .filter(|tile| tile.tile_type == TileType::Floor)
            .count();
        assert!(floor_count > 0);
    }
    
    #[test]
    fn test_l_corridor_carving() {
        let generator = RoomCorridorGenerator::new();
        let mut level = Level::new(0, 20, 20);
        let config = GenerationConfig::for_testing(12345);
        
        let start = Position::new(5, 5);
        let end = Position::new(15, 15);
        
        let result = generator.carve_l_corridor(&mut level, start, end, &config);
        assert!(result.is_ok());
        
        // Check that start and end positions are now floor
        assert_eq!(level.get_tile(start).unwrap().tile_type, TileType::Floor);
        assert_eq!(level.get_tile(end).unwrap().tile_type, TileType::Floor);
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