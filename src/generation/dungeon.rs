//! # Dungeon Generation
//!
//! Procedural dungeon layout generation using room-and-corridor algorithms.
//!
//! This module implements sophisticated dungeon generation algorithms that create
//! interesting, connected layouts. The system supports various generation strategies
//! and can be enhanced by the LLDM for unique architectural features.

use crate::generation::utils;
use crate::{ThatchError, ThatchResult};
use crate::game::{Level, Position, Tile, TileType, World};
use crate::generation::{GenerationConfig, Generator, Room, RoomType};
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
/// This generator creates entire 3D dungeons by:
/// 1. Placing stairs on all 26 floors first to ensure vertical connectivity
/// 2. Placing rooms around stairs and randomly on each floor
/// 3. Starting with all non-room spaces as open floor
/// 4. Progressively adding walls while maintaining connectivity
/// 5. Ensuring stairs are always connected within each level
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
    /// Whether to generate all 26 floors at once (3D generation)
    pub generate_all_floors: bool,
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
            max_connectivity_failures: 1000,
            max_placement_attempts: 100,
            ensure_connectivity: true,
            generate_all_floors: true,
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
            generate_all_floors: true,
        }
    }

    /// Creates a generator optimized for testing.
    pub fn for_testing() -> Self {
        Self {
            room_placement_strategy: RoomPlacementStrategy::Random,
            max_connectivity_failures: 50,
            max_placement_attempts: 50,
            ensure_connectivity: true,
            generate_all_floors: false, // Single floor for testing
        }
    }

    /// Creates a generator for detailed, complex dungeons.
    pub fn for_detailed_generation() -> Self {
        Self {
            room_placement_strategy: RoomPlacementStrategy::NoiseGuided,
            max_connectivity_failures: 1500,
            max_placement_attempts: 200,
            ensure_connectivity: true,
            generate_all_floors: true,
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
    #[allow(dead_code)]
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
        
        // For 3D generation, be less aggressive with wall placement
        let max_failures = if self.generate_all_floors { 
            self.max_connectivity_failures / 2 
        } else { 
            self.max_connectivity_failures 
        };
        
        // Also limit the number of positions we try to convert
        let max_walls_to_place = available_positions.len() / 3; // Only convert 1/3 of available positions
        let mut walls_placed = 0;

        while connectivity_failures < max_failures
            && !available_positions.is_empty()
            && walls_placed < max_walls_to_place
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
                walls_placed += 1;
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

    /// Fills all unreachable floor tiles with walls using flood fill from spawn position.
    ///
    /// This ensures that only reachable areas remain as floor tiles, creating a more
    /// compact and connected dungeon layout.
    fn fill_unreachable_areas(&self, level: &mut Level) -> ThatchResult<()> {
        let spawn_pos = level.player_spawn;
        
        // Find all reachable floor tiles using flood fill
        let reachable_tiles = self.flood_fill_reachable(level, spawn_pos)?;
        
        // Ensure we have at least some reachable tiles
        if reachable_tiles.is_empty() {
            // This is a critical error - the spawn position should always be reachable
            return Err(ThatchError::GenerationFailed(
                format!("Spawn position {:?} is not reachable", spawn_pos)
            ));
        }
        
        // Convert all unreachable floor tiles to walls, but preserve stairs
        for y in 0..level.height as i32 {
            for x in 0..level.width as i32 {
                let pos = Position::new(x, y);
                
                if let Some(tile) = level.get_tile(pos) {
                    // Don't convert stairs to walls, even if unreachable
                    match tile.tile_type {
                        TileType::StairsUp | TileType::StairsDown => {
                            // Keep stairs as they are
                            continue;
                        }
                        _ => {
                            // If it's a passable tile but not reachable, convert to wall
                            if tile.tile_type.is_passable() && !reachable_tiles.contains(&pos) {
                                level.set_tile(pos, Tile::wall())?;
                            }
                        }
                    }
                }
            }
        }
        
        Ok(())
    }

    /// Performs flood fill from the spawn position to find all reachable tiles.
    ///
    /// Uses breadth-first search to explore all passable tiles reachable from the spawn.
    fn flood_fill_reachable(&self, level: &Level, start: Position) -> ThatchResult<HashSet<Position>> {
        let mut reachable = HashSet::new();
        let mut queue = VecDeque::new();
        
        // Start from spawn position if it's passable
        if let Some(start_tile) = level.get_tile(start) {
            if start_tile.tile_type.is_passable() {
                queue.push_back(start);
                reachable.insert(start);
            } else {
                // If spawn is not passable, return empty set (shouldn't happen in normal generation)
                return Ok(reachable);
            }
        } else {
            return Ok(reachable);
        }
        
        // Breadth-first search to find all reachable tiles
        while let Some(current) = queue.pop_front() {
            // Check all cardinal neighbors
            for neighbor in current.cardinal_adjacent_positions() {
                // Skip if already visited
                if reachable.contains(&neighbor) {
                    continue;
                }
                
                // Skip if out of bounds
                if !level.is_valid_position(neighbor) {
                    continue;
                }
                
                // Check if tile is passable
                if let Some(tile) = level.get_tile(neighbor) {
                    if tile.tile_type.is_passable() {
                        reachable.insert(neighbor);
                        queue.push_back(neighbor);
                    }
                }
            }
        }
        
        Ok(reachable)
    }

    /// Creates special stair rooms and places stairs to connect between levels.
    /// Treats stairs as single-cell "rooms" for proper connectivity.
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

        // Create stairs up room - single cell treated as a special room
        let stairs_up_pos = self.find_stairs_position(level, rooms, true, rng)?;
        level.set_tile(stairs_up_pos, Tile::new(TileType::StairsUp))?;
        level.stairs_up_position = Some(stairs_up_pos);
        
        // Always set player spawn to stairs up position
        level.player_spawn = stairs_up_pos;

        // Create stairs down room if not the deepest level
        if level.id < 25 { // Don't add stairs down on final level
            let stairs_down_pos = self.find_stairs_position_avoiding(level, rooms, false, stairs_up_pos, rng)?;
            level.set_tile(stairs_down_pos, Tile::new(TileType::StairsDown))?;
            level.stairs_down_position = Some(stairs_down_pos);
            
            // CRITICAL: Ensure there's a path between up and down stairs
            if !self.has_path(level, stairs_up_pos, stairs_down_pos)? {
                // If no path exists, clear a corridor between them
                self.create_stair_connection(level, stairs_up_pos, stairs_down_pos)?;
            }
        }

        Ok(())
    }

    /// Finds appropriate position for stairs, treating them as special single-cell rooms.
    fn find_stairs_position(
        &self,
        level: &Level,
        rooms: &[Room],
        _is_up_stairs: bool,
        rng: &mut StdRng,
    ) -> ThatchResult<Position> {
        // Try to find a good position for stairs
        // Prefer positions that are accessible but not in the center of large rooms
        
        let mut candidates = Vec::new();
        
        // Look for floor positions that are:
        // 1. Adjacent to at least one wall (for interesting placement)
        // 2. Not in the exact center of rooms (to avoid blocking room flow)
        // 3. Accessible from the main dungeon area
        
        for room in rooms {
            let room_positions = room.floor_positions();
            for pos in room_positions {
                if self.is_good_stair_position(level, pos) {
                    candidates.push(pos);
                }
            }
        }
        
        // If we have candidates, pick one randomly
        if !candidates.is_empty() {
            let index = rng.gen_range(0..candidates.len());
            return Ok(candidates[index]);
        }
        
        // Fallback: use center of first room
        if !rooms.is_empty() {
            return Ok(rooms[0].center());
        }
        
        // Final fallback: use level center
        Ok(Position::new(level.width as i32 / 2, level.height as i32 / 2))
    }
    
    /// Checks if a position is suitable for stairs placement.
    fn is_good_stair_position(&self, level: &Level, pos: Position) -> bool {
        // Must be a floor tile
        if let Some(tile) = level.get_tile(pos) {
            if tile.tile_type != TileType::Floor {
                return false;
            }
        } else {
            return false;
        }
        
        // Check if it has at least one adjacent wall (makes it feel more natural)
        let adjacent_positions = pos.adjacent_positions();
        let has_adjacent_wall = adjacent_positions.iter().any(|&adj_pos| {
            if let Some(tile) = level.get_tile(adj_pos) {
                tile.tile_type == TileType::Wall
            } else {
                true // Out of bounds counts as wall
            }
        });
        
        has_adjacent_wall
    }
    
    /// Finds appropriate position for stairs while avoiding a specific position.
    /// This ensures up and down stairs are placed in different locations.
    fn find_stairs_position_avoiding(
        &self,
        level: &Level,
        rooms: &[Room],
        _is_up_stairs: bool,
        avoid_position: Position,
        rng: &mut StdRng,
    ) -> ThatchResult<Position> {
        // Try to find a good position for stairs, avoiding the specified position
        let mut candidates = Vec::new();
        
        for room in rooms {
            let room_positions = room.floor_positions();
            for pos in room_positions {
                if self.is_good_stair_position(level, pos) && pos != avoid_position {
                    // Prefer positions that are further away from the avoid_position
                    let distance = pos.manhattan_distance(avoid_position);
                    if distance >= 5 { // Minimum distance between stairs
                        candidates.push(pos);
                    }
                }
            }
        }
        
        // If we have good candidates, pick one randomly
        if !candidates.is_empty() {
            let index = rng.gen_range(0..candidates.len());
            return Ok(candidates[index]);
        }
        
        // Fallback: find any position different from avoid_position
        let mut fallback_candidates = Vec::new();
        for room in rooms {
            let room_positions = room.floor_positions();
            for pos in room_positions {
                if pos != avoid_position {
                    fallback_candidates.push(pos);
                }
            }
        }
        
        if !fallback_candidates.is_empty() {
            let index = rng.gen_range(0..fallback_candidates.len());
            return Ok(fallback_candidates[index]);
        }
        
        // Final fallback: use a position different from avoid
        let fallback = Position::new(
            if avoid_position.x > level.width as i32 / 2 { 
                level.width as i32 / 4 
            } else { 
                (level.width as i32 * 3) / 4 
            },
            if avoid_position.y > level.height as i32 / 2 { 
                level.height as i32 / 4 
            } else { 
                (level.height as i32 * 3) / 4 
            }
        );
        
        Ok(fallback)
    }
    
    /// Creates a direct connection between two stair positions if none exists.
    /// Uses a simple line-drawing algorithm to carve a corridor.
    fn create_stair_connection(
        &self,
        level: &mut Level,
        start: Position,
        end: Position,
    ) -> ThatchResult<()> {
        // Use Bresenham's line algorithm to draw a path between stairs
        let positions = self.line_between_points(start, end);
        
        // Clear all positions along the path
        for pos in positions {
            if level.is_valid_position(pos) {
                // Don't overwrite the stairs themselves
                if let Some(tile) = level.get_tile(pos) {
                    match tile.tile_type {
                        TileType::StairsUp | TileType::StairsDown => {
                            // Leave stairs as they are
                            continue;
                        }
                        _ => {
                            // Clear everything else to floor
                            level.set_tile(pos, Tile::floor())?;
                        }
                    }
                }
            }
        }
        
        // Also clear a 1-tile buffer around the path for better connectivity
        let path_positions = self.line_between_points(start, end);
        for pos in path_positions {
            for adjacent in pos.cardinal_adjacent_positions() {
                if level.is_valid_position(adjacent) {
                    // Only clear if it's a wall and not on the level boundary
                    if adjacent.x > 0 && adjacent.y > 0 && 
                       adjacent.x < (level.width as i32 - 1) && 
                       adjacent.y < (level.height as i32 - 1) {
                        if let Some(tile) = level.get_tile(adjacent) {
                            if tile.tile_type == TileType::Wall {
                                level.set_tile(adjacent, Tile::floor())?;
                            }
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Generates points along a line between two positions using Bresenham's algorithm.
    fn line_between_points(&self, start: Position, end: Position) -> Vec<Position> {
        let mut points = Vec::new();
        
        let mut x0 = start.x;
        let mut y0 = start.y;
        let x1 = end.x;
        let y1 = end.y;
        
        let dx = (x1 - x0).abs();
        let dy = (y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx - dy;
        
        loop {
            points.push(Position::new(x0, y0));
            
            if x0 == x1 && y0 == y1 {
                break;
            }
            
            let e2 = 2 * err;
            
            if e2 > -dy {
                err -= dy;
                x0 += sx;
            }
            
            if e2 < dx {
                err += dx;
                y0 += sy;
            }
        }
        
        points
    }

    /// Generates a complete 3D dungeon with all 26 floors at once.
    ///
    /// This method creates all levels with aligned stairs and proper connectivity:
    /// 1. Places stairs on all floors first to ensure vertical alignment
    /// 2. Creates rooms around stairs and randomly places additional rooms
    /// 3. Applies the standard generation algorithm to each floor
    pub fn generate_complete_dungeon(&self, config: &GenerationConfig, rng: &mut StdRng) -> ThatchResult<World> {
        let mut world = World::new(config.seed);
        
        // Step 1: Generate stairs positions for all 26 floors
        let stair_positions = self.generate_stair_layout(config, rng)?;
        
        // Step 2: Generate each floor with pre-placed stairs
        for floor_id in 0..26 {
            let level = self.generate_floor_with_stairs(
                floor_id, 
                &stair_positions, 
                config, 
                rng
            )?;
            
            world.add_level(level);
        }
        
        Ok(world)
    }
    
    /// Generates the stair layout for all 26 floors.
    ///
    /// Returns a map of floor_id -> (stairs_up_pos, stairs_down_pos)
    /// Ensures vertical alignment between floors.
    fn generate_stair_layout(&self, _config: &GenerationConfig, rng: &mut StdRng) -> ThatchResult<HashMap<u32, (Option<Position>, Option<Position>)>> {
        let mut stair_positions = HashMap::new();
        
        // Determine level dimensions (consistent across all floors)
        let level_width = 80;  // Fixed reasonable size
        let level_height = 50;
        
        // Generate stairs positions ensuring vertical alignment
        for floor_id in 0..26 {
            let stairs_up = if floor_id > 0 {
                // Use the down stairs position from the floor above
                stair_positions.get(&(floor_id - 1))
                    .and_then(|(_, down_pos)| *down_pos)
            } else {
                None // No up stairs on floor 0
            };
            
            let stairs_down = if floor_id < 25 {
                // Generate a new down stairs position for this floor
                let x = rng.gen_range(5..(level_width as i32 - 5));
                let y = rng.gen_range(5..(level_height as i32 - 5));
                
                // Ensure down stairs is not too close to up stairs
                let pos = if let Some(up_pos) = stairs_up {
                    let mut attempts = 0;
                    let mut candidate_pos = Position::new(x, y);
                    
                    while attempts < 20 && candidate_pos.manhattan_distance(up_pos) < 10 {
                        let new_x = rng.gen_range(5..(level_width as i32 - 5));
                        let new_y = rng.gen_range(5..(level_height as i32 - 5));
                        candidate_pos = Position::new(new_x, new_y);
                        attempts += 1;
                    }
                    
                    candidate_pos
                } else {
                    Position::new(x, y)
                };
                
                Some(pos)
            } else {
                None // No down stairs on floor 25
            };
            
            stair_positions.insert(floor_id, (stairs_up, stairs_down));
        }
        
        Ok(stair_positions)
    }
    
    /// Generates a single floor with pre-placed stairs.
    fn generate_floor_with_stairs(
        &self,
        floor_id: u32,
        stair_positions: &HashMap<u32, (Option<Position>, Option<Position>)>,
        config: &GenerationConfig,
        rng: &mut StdRng,
    ) -> ThatchResult<Level> {
        let level_width = 80;
        let level_height = 50;
        let mut level = Level::new(floor_id, level_width, level_height);
        
        // Get stairs positions for this floor
        let (stairs_up_pos, stairs_down_pos) = stair_positions.get(&floor_id)
            .cloned()
            .unwrap_or((None, None));
        
        // Set stairs positions in level
        level.stairs_up_position = stairs_up_pos;
        level.stairs_down_position = stairs_down_pos;
        
        // Step 1: Create rooms around stairs and additional random rooms
        let mut rooms = Vec::new();
        let mut room_id = 0;
        
        // Create room around stairs up (if exists)
        if let Some(up_pos) = stairs_up_pos {
            let room = self.create_room_around_position(room_id, up_pos, config, rng, &level)?;
            rooms.push(room);
            room_id += 1;
        }
        
        // Create room around stairs down (if exists)
        if let Some(down_pos) = stairs_down_pos {
            let room = self.create_room_around_position(room_id, down_pos, config, rng, &level)?;
            rooms.push(room);
            room_id += 1;
        }
        
        // Add 2-5 additional random rooms, with more attempts if we don't have many rooms yet
        let target_additional_rooms = rng.gen_range(2..=5);
        let mut attempts = 0;
        let max_attempts = target_additional_rooms * 10; // More attempts per room
        
        while rooms.len() < (target_additional_rooms + if stairs_up_pos.is_some() { 1 } else { 0 } + if stairs_down_pos.is_some() { 1 } else { 0 }) 
              && attempts < max_attempts {
            if let Some(room) = self.try_place_room_overlapping(&level, config, rng, room_id)? {
                rooms.push(room);
                room_id += 1;
            }
            attempts += 1;
        }
        
        // If we still have very few rooms, force place at least one room
        if rooms.is_empty() {
            // Force place a room at the center of the level
            let center_room = Room::new(
                room_id,
                Position::new(level_width as i32 / 2 - 5, level_height as i32 / 2 - 5),
                10,
                10,
                RoomType::Normal,
            );
            rooms.push(center_room);
        }
        
        // Set player spawn to stairs up position, or center of first room if no stairs up
        level.player_spawn = if let Some(up_pos) = stairs_up_pos {
            up_pos
        } else {
            // For floor 0, spawn in the center of the first room
            rooms[0].center()
        };
        
        // Step 2: Initialize level with rooms and open floor everywhere else
        self.initialize_level_with_rooms(&mut level, &rooms)?;
        
        // Step 3: Place stairs tiles
        if let Some(up_pos) = stairs_up_pos {
            level.set_tile(up_pos, Tile::new(TileType::StairsUp))?;
        }
        if let Some(down_pos) = stairs_down_pos {
            level.set_tile(down_pos, Tile::new(TileType::StairsDown))?;
        }
        
        // Step 4: Progressively add walls while maintaining connectivity
        // Note: This step can be aggressive, so we'll limit it for 3D generation
        self.progressive_wall_placement(&mut level, &rooms, rng)?;
        
        // Step 5: Ensure stairs are connected if both exist
        if let (Some(up_pos), Some(down_pos)) = (stairs_up_pos, stairs_down_pos) {
            if !self.has_path(&level, up_pos, down_pos)? {
                self.create_stair_connection(&mut level, up_pos, down_pos)?;
            }
        }
        
        // Step 6: Fill unreachable areas with walls (disabled for now to debug)
        // NOTE: This step might be too aggressive for 3D generation
        // self.fill_unreachable_areas(&mut level)?;
        
        // Final validation with better error reporting
        let floor_count = level.tiles.iter()
            .flat_map(|row| row.iter())
            .filter(|tile| tile.tile_type.is_passable())
            .count();
            
        if floor_count == 0 {
            return Err(ThatchError::GenerationFailed(
                format!("Floor {} generation resulted in no passable tiles. Rooms: {}, Spawn: {:?}, Up stairs: {:?}, Down stairs: {:?}", 
                    floor_id, rooms.len(), level.player_spawn, stairs_up_pos, stairs_down_pos)
            ));
        }
        
        utils::validate_level(&level)?;
        
        Ok(level)
    }
    
    /// Creates a room around a specific position (usually stairs).
    fn create_room_around_position(
        &self,
        room_id: u32,
        center: Position,
        config: &GenerationConfig,
        rng: &mut StdRng,
        level: &Level,
    ) -> ThatchResult<Room> {
        let room_width = rng.gen_range(config.min_room_size..=config.max_room_size);
        let room_height = rng.gen_range(config.min_room_size..=config.max_room_size);
        
        // Calculate top-left position to center the room around the given position
        let top_left_x = (center.x - room_width as i32 / 2).max(1);
        let top_left_y = (center.y - room_height as i32 / 2).max(1);
        
        // Ensure room fits within level bounds
        let adjusted_x = top_left_x.min(level.width as i32 - room_width as i32 - 1);
        let adjusted_y = top_left_y.min(level.height as i32 - room_height as i32 - 1);
        
        let room_type = self.determine_room_type(room_id, config, rng);
        
        Ok(Room::new(
            room_id,
            Position::new(adjusted_x, adjusted_y),
            room_width,
            room_height,
            room_type,
        ))
    }
}

impl Generator<Level> for RoomCorridorGenerator {
    fn generate(&self, config: &GenerationConfig, rng: &mut StdRng) -> ThatchResult<Level> {
        if self.generate_all_floors {
            // For 3D generation, just return the first floor of a complete dungeon
            let world = self.generate_complete_dungeon(config, rng)?;
            return world.get_level(0)
                .cloned()
                .ok_or_else(|| ThatchError::GenerationFailed("Failed to get first level from generated world".to_string()));
        }
        
        // Original single-level generation for testing and specific use cases
        let estimated_width = ((config.max_rooms * config.max_room_size * 2) as f64).sqrt() as u32;
        let estimated_height = estimated_width;
        let width = estimated_width.clamp(50, 200); // Reasonable bounds
        let height = estimated_height.clamp(50, 200);
        let mut level = Level::new(0, width, height);

        // Step 1: Place rooms (overlapping allowed)
        let rooms = self.place_rooms(&mut level, config, rng)?;

        // Step 2: Initialize level with rooms and open floor everywhere else
        self.initialize_level_with_rooms(&mut level, &rooms)?;

        // Step 3: Progressively add walls while maintaining connectivity
        self.progressive_wall_placement(&mut level, &rooms, rng)?;

        // Player spawn will be set in add_stairs method to stairs up position

        // Step 4: Add stairs
        self.add_stairs(&mut level, &rooms, config, rng)?;

        // Step 5: Fill unreachable areas with walls
        self.fill_unreachable_areas(&mut level)?;

        // Apply LLDM enhancements if enabled
        if config.use_lldm {
            // LLDM enhancement would be implemented here
        }

        // Final validation
        utils::validate_level(&level)?;
        
        // Critical: Final check that stairs are connected if both exist
        if let (Some(stairs_up), Some(stairs_down)) = (level.stairs_up_position, level.stairs_down_position) {
            if !self.has_path(&level, stairs_up, stairs_down)? {
                // This should not happen if our algorithm is correct, but just in case
                self.create_stair_connection(&mut level, stairs_up, stairs_down)?;
            }
        }

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

/// Trait for generating complete dungeon worlds.
pub trait WorldGenerator {
    /// Generates a complete multi-level world.
    fn generate_world(&self, config: &GenerationConfig, rng: &mut StdRng) -> ThatchResult<World>;
    
    /// Validates a generated world.
    fn validate_world(&self, world: &World, config: &GenerationConfig) -> ThatchResult<()>;
}

impl WorldGenerator for RoomCorridorGenerator {
    fn generate_world(&self, config: &GenerationConfig, rng: &mut StdRng) -> ThatchResult<World> {
        self.generate_complete_dungeon(config, rng)
    }
    
    fn validate_world(&self, world: &World, _config: &GenerationConfig) -> ThatchResult<()> {
        // Validate each level in the world
        for level in world.levels.values() {
            utils::validate_level(level)?;
        }
        
        // Validate stair connectivity between levels
        for level_id in 0..25 {
            if let (Some(current_level), Some(next_level)) = (world.get_level(level_id), world.get_level(level_id + 1)) {
                // Check that down stairs on current level align with up stairs on next level
                if let (Some(down_pos), Some(up_pos)) = (current_level.stairs_down_position, next_level.stairs_up_position) {
                    if down_pos != up_pos {
                        return Err(ThatchError::GenerationFailed(
                            format!("Stair misalignment between levels {} and {}: down at {:?}, up at {:?}", 
                                level_id, level_id + 1, down_pos, up_pos)
                        ));
                    }
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

    #[test]
    fn test_room_corridor_generator_creation() {
        let generator = RoomCorridorGenerator::new();
        assert_eq!(
            generator.room_placement_strategy,
            RoomPlacementStrategy::Random
        );
        assert_eq!(generator.max_connectivity_failures, 1000);
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

    #[test]
    fn test_fill_unreachable_areas() {
        let generator = RoomCorridorGenerator::new();
        let mut level = Level::new(0, 10, 10);
        
        // Create a connected area around spawn
        let spawn_pos = Position::new(5, 5);
        level.player_spawn = spawn_pos;
        level.set_tile(spawn_pos, Tile::floor()).unwrap();
        level.set_tile(Position::new(5, 6), Tile::floor()).unwrap();
        level.set_tile(Position::new(6, 5), Tile::floor()).unwrap();
        
        // Create an isolated area that should be filled
        level.set_tile(Position::new(2, 2), Tile::floor()).unwrap();
        level.set_tile(Position::new(2, 3), Tile::floor()).unwrap();
        
        // Fill unreachable areas
        generator.fill_unreachable_areas(&mut level).unwrap();
        
        // Check that spawn area is still floor
        assert_eq!(level.get_tile(spawn_pos).unwrap().tile_type, TileType::Floor);
        assert_eq!(level.get_tile(Position::new(5, 6)).unwrap().tile_type, TileType::Floor);
        assert_eq!(level.get_tile(Position::new(6, 5)).unwrap().tile_type, TileType::Floor);
        
        // Check that isolated area was filled with walls
        assert_eq!(level.get_tile(Position::new(2, 2)).unwrap().tile_type, TileType::Wall);
        assert_eq!(level.get_tile(Position::new(2, 3)).unwrap().tile_type, TileType::Wall);
    }

    #[test]
    fn test_flood_fill_reachable() {
        let generator = RoomCorridorGenerator::new();
        let mut level = Level::new(0, 10, 10);
        
        // Create a connected area
        let start_pos = Position::new(5, 5);
        level.set_tile(start_pos, Tile::floor()).unwrap();
        level.set_tile(Position::new(5, 6), Tile::floor()).unwrap();
        level.set_tile(Position::new(6, 5), Tile::floor()).unwrap();
        level.set_tile(Position::new(4, 5), Tile::floor()).unwrap();
        
        // Create isolated floor tiles
        level.set_tile(Position::new(2, 2), Tile::floor()).unwrap();
        
        let reachable = generator.flood_fill_reachable(&level, start_pos).unwrap();
        
        // Should include connected tiles
        assert!(reachable.contains(&start_pos));
        assert!(reachable.contains(&Position::new(5, 6)));
        assert!(reachable.contains(&Position::new(6, 5)));
        assert!(reachable.contains(&Position::new(4, 5)));
        
        // Should NOT include isolated tiles
        assert!(!reachable.contains(&Position::new(2, 2)));
        
        // Should have exactly 4 reachable tiles
        assert_eq!(reachable.len(), 4);
    }

    #[test]
    fn test_stair_connectivity() {
        let generator = RoomCorridorGenerator::for_testing();
        let config = GenerationConfig::for_testing(54321);
        let mut rng = utils::create_rng(&config);

        // Generate a level
        let level = generator.generate(&config, &mut rng).unwrap();
        
        // Check if level has both up and down stairs
        if let (Some(stairs_up), Some(stairs_down)) = 
            (level.stairs_up_position, level.stairs_down_position) {
            
            // Verify there's a path between them
            assert!(generator.has_path(&level, stairs_up, stairs_down).unwrap(),
                   "Stairs up and down should be connected by a path");
            
            // Verify stairs are not in the same position
            assert_ne!(stairs_up, stairs_down, "Stairs should be in different positions");
        }
    }

    #[test]
    fn test_line_between_points() {
        let generator = RoomCorridorGenerator::new();
        
        // Test horizontal line
        let points = generator.line_between_points(
            Position::new(1, 5), 
            Position::new(5, 5)
        );
        assert_eq!(points.len(), 5);
        assert!(points.contains(&Position::new(1, 5)));
        assert!(points.contains(&Position::new(3, 5)));
        assert!(points.contains(&Position::new(5, 5)));
        
        // Test vertical line
        let points = generator.line_between_points(
            Position::new(3, 1), 
            Position::new(3, 4)
        );
        assert_eq!(points.len(), 4);
        assert!(points.contains(&Position::new(3, 1)));
        assert!(points.contains(&Position::new(3, 4)));
        
        // Test diagonal line
        let points = generator.line_between_points(
            Position::new(0, 0), 
            Position::new(2, 2)
        );
        assert_eq!(points.len(), 3);
        assert!(points.contains(&Position::new(0, 0)));
        assert!(points.contains(&Position::new(1, 1)));
        assert!(points.contains(&Position::new(2, 2)));
    }

    #[test]
    fn test_stair_connection_creation() {
        let generator = RoomCorridorGenerator::new();
        let mut level = Level::new(0, 20, 20);
        
        // Fill level with walls initially
        for y in 0..20 {
            for x in 0..20 {
                let pos = Position::new(x as i32, y as i32);
                level.set_tile(pos, Tile::wall()).unwrap();
            }
        }
        
        // Place stairs at specific positions
        let stairs_up = Position::new(2, 2);
        let stairs_down = Position::new(17, 17);
        level.set_tile(stairs_up, Tile::new(TileType::StairsUp)).unwrap();
        level.set_tile(stairs_down, Tile::new(TileType::StairsDown)).unwrap();
        
        // Create connection
        generator.create_stair_connection(&mut level, stairs_up, stairs_down).unwrap();
        
        // Verify path exists
        assert!(generator.has_path(&level, stairs_up, stairs_down).unwrap(),
               "Connection should create a valid path between stairs");
    }

    #[test]
    fn test_3d_stair_layout_generation() {
        let generator = RoomCorridorGenerator::new();
        let config = GenerationConfig::for_testing(12345);
        let mut rng = utils::create_rng(&config);
        
        let stair_positions = generator.generate_stair_layout(&config, &mut rng).unwrap();
        
        // Should have positions for all 26 floors
        assert_eq!(stair_positions.len(), 26);
        
        // Floor 0 should have no up stairs but should have down stairs
        let (up_0, down_0) = stair_positions.get(&0).unwrap();
        assert!(up_0.is_none());
        assert!(down_0.is_some());
        
        // Floor 25 should have up stairs but no down stairs
        let (up_25, down_25) = stair_positions.get(&25).unwrap();
        assert!(up_25.is_some());
        assert!(down_25.is_none());
        
        // Middle floors should have both up and down stairs
        for floor_id in 1..25 {
            let (up_pos, down_pos) = stair_positions.get(&floor_id).unwrap();
            assert!(up_pos.is_some(), "Floor {} should have up stairs", floor_id);
            assert!(down_pos.is_some(), "Floor {} should have down stairs", floor_id);
        }
        
        // Verify stair alignment: down stairs on floor N should match up stairs on floor N+1
        for floor_id in 0..25 {
            let (_, down_pos) = stair_positions.get(&floor_id).unwrap();
            let (up_pos_next, _) = stair_positions.get(&(floor_id + 1)).unwrap();
            
            assert_eq!(down_pos, up_pos_next, 
                      "Stairs should align between floors {} and {}", floor_id, floor_id + 1);
        }
    }

    #[test]
    fn test_complete_dungeon_generation() {
        let generator = RoomCorridorGenerator::new();
        let config = GenerationConfig::for_testing(54321);
        let mut rng = utils::create_rng(&config);
        
        let world = generator.generate_complete_dungeon(&config, &mut rng).unwrap();
        
        // Should have all 26 levels
        assert_eq!(world.levels.len(), 26);
        
        // All levels should exist
        for level_id in 0..26 {
            assert!(world.get_level(level_id).is_some(), "Level {} should exist", level_id);
        }
        
        // Verify stair connectivity across all levels
        for level_id in 0..25 {
            let current_level = world.get_level(level_id).unwrap();
            let next_level = world.get_level(level_id + 1).unwrap();
            
            // Down stairs on current level should match up stairs on next level
            if let (Some(down_pos), Some(up_pos)) = (current_level.stairs_down_position, next_level.stairs_up_position) {
                assert_eq!(down_pos, up_pos, 
                          "Stair positions should match between levels {} and {}", level_id, level_id + 1);
            }
        }
    }

    #[test]
    fn test_world_generator_trait() {
        let generator = RoomCorridorGenerator::new();
        let config = GenerationConfig::for_testing(98765);
        let mut rng = utils::create_rng(&config);
        
        // Test world generation through trait
        let world = generator.generate_world(&config, &mut rng).unwrap();
        
        // Test world validation through trait
        assert!(generator.validate_world(&world, &config).is_ok());
        
        // Should have 26 levels
        assert_eq!(world.levels.len(), 26);
    }

    #[test]
    fn test_room_around_position() {
        let generator = RoomCorridorGenerator::new();
        let config = GenerationConfig::for_testing(11111);
        let mut rng = utils::create_rng(&config);
        let level = Level::new(0, 50, 40);
        
        let center_pos = Position::new(25, 20);
        let room = generator.create_room_around_position(1, center_pos, &config, &mut rng, &level).unwrap();
        
        // Room should contain the center position
        assert!(room.contains(center_pos), "Room should contain the center position");
        
        // Room should be within level bounds
        assert!(room.top_left.x >= 1);
        assert!(room.top_left.y >= 1);
        assert!(room.top_left.x + (room.width as i32) < (level.width as i32) - 1);
        assert!(room.top_left.y + (room.height as i32) < (level.height as i32) - 1);
    }

    #[test] 
    fn test_single_vs_3d_generation() {
        let config = GenerationConfig::for_testing(22222);
        let mut rng = utils::create_rng(&config);
        
        // Test single floor generation
        let single_generator = RoomCorridorGenerator::for_testing(); // generate_all_floors = false
        let single_level = single_generator.generate(&config, &mut rng).unwrap();
        assert_eq!(single_level.id, 0);
        
        // Test 3D generation via single level interface
        let mut rng2 = utils::create_rng(&config);
        let multi_generator = RoomCorridorGenerator::new(); // generate_all_floors = true
        let first_level = multi_generator.generate(&config, &mut rng2).unwrap();
        assert_eq!(first_level.id, 0);
        
        // Both should be valid levels
        assert!(utils::validate_level(&single_level).is_ok());
        assert!(utils::validate_level(&first_level).is_ok());
    }

    #[test]
    fn test_floor_0_generation_debug() {
        let generator = RoomCorridorGenerator::new();
        let config = GenerationConfig::for_testing(99999);
        let mut rng = utils::create_rng(&config);
        
        // Generate stair layout
        let stair_positions = generator.generate_stair_layout(&config, &mut rng).unwrap();
        
        // Test generating just floor 0
        let floor_0_result = generator.generate_floor_with_stairs(0, &stair_positions, &config, &mut rng);
        
        match floor_0_result {
            Ok(level) => {
                // Count passable tiles
                let passable_count = level.tiles.iter()
                    .flat_map(|row| row.iter())
                    .filter(|tile| tile.tile_type.is_passable())
                    .count();
                println!("Floor 0 generated successfully with {} passable tiles", passable_count);
                assert!(passable_count > 0, "Floor 0 should have passable tiles");
            }
            Err(e) => {
                panic!("Floor 0 generation failed: {}", e);
            }
        }
    }

    #[test]
    fn test_stair_alignment_consistency() {
        let generator = RoomCorridorGenerator::new();
        let config = GenerationConfig::for_testing(44444);
        let mut rng = utils::create_rng(&config);
        
        // Generate multiple stair layouts and verify consistency
        for seed_offset in 0..5 {
            let mut test_rng = utils::create_rng(&GenerationConfig::for_testing(44444 + seed_offset));
            let stair_positions = generator.generate_stair_layout(&config, &mut test_rng).unwrap();
            
            // Verify basic properties
            assert_eq!(stair_positions.len(), 26);
            
            // Check first and last floors
            let (up_0, down_0) = stair_positions.get(&0).unwrap();
            assert!(up_0.is_none());
            assert!(down_0.is_some());
            
            let (up_25, down_25) = stair_positions.get(&25).unwrap();
            assert!(up_25.is_some());
            assert!(down_25.is_none());
            
            // Verify alignment
            for floor_id in 0..25 {
                let (_, down_current) = stair_positions.get(&floor_id).unwrap();
                let (up_next, _) = stair_positions.get(&(floor_id + 1)).unwrap();
                assert_eq!(down_current, up_next, "Stairs misaligned between floors {} and {}", floor_id, floor_id + 1);
            }
        }
    }

    #[test]
    fn test_room_around_position_edge_cases() {
        let generator = RoomCorridorGenerator::new();
        let config = GenerationConfig::for_testing(55555);
        let mut rng = utils::create_rng(&config);
        let level = Level::new(0, 50, 40);
        
        // Test room around position near level boundaries
        let edge_positions = vec![
            Position::new(2, 2),    // Near top-left
            Position::new(47, 2),   // Near top-right
            Position::new(2, 37),   // Near bottom-left
            Position::new(47, 37),  // Near bottom-right
        ];
        
        for pos in edge_positions {
            let room = generator.create_room_around_position(1, pos, &config, &mut rng, &level).unwrap();
            
            // Room should be within bounds
            assert!(room.top_left.x >= 1);
            assert!(room.top_left.y >= 1);
            assert!(room.top_left.x + (room.width as i32) < (level.width as i32) - 1);
            assert!(room.top_left.y + (room.height as i32) < (level.height as i32) - 1);
            
            // Room should contain the target position
            assert!(room.contains(pos), "Room should contain target position {:?}", pos);
        }
    }

    #[test]
    fn test_progressive_wall_placement_3d_vs_single() {
        let config = GenerationConfig::for_testing(66666);
        let mut rng = utils::create_rng(&config);
        
        // Create identical starting levels
        let mut level_3d = Level::new(0, 30, 20);
        let mut level_single = Level::new(0, 30, 20);
        
        // Place some test rooms
        let rooms = vec![
            Room::new(0, Position::new(5, 5), 8, 6, RoomType::Normal),
            Room::new(1, Position::new(15, 10), 6, 8, RoomType::Normal),
        ];
        
        // Initialize both levels identically
        let generator_3d = RoomCorridorGenerator::new(); // generate_all_floors = true
        let generator_single = RoomCorridorGenerator::for_testing(); // generate_all_floors = false
        
        generator_3d.initialize_level_with_rooms(&mut level_3d, &rooms).unwrap();
        generator_single.initialize_level_with_rooms(&mut level_single, &rooms).unwrap();
        
        // Apply wall placement
        let mut rng_3d = utils::create_rng(&config);
        let mut rng_single = utils::create_rng(&config);
        
        generator_3d.progressive_wall_placement(&mut level_3d, &rooms, &mut rng_3d).unwrap();
        generator_single.progressive_wall_placement(&mut level_single, &rooms, &mut rng_single).unwrap();
        
        // Count walls in each
        let count_walls = |level: &Level| {
            level.tiles.iter()
                .flat_map(|row| row.iter())
                .filter(|tile| tile.tile_type == TileType::Wall)
                .count()
        };
        
        let walls_3d = count_walls(&level_3d);
        let walls_single = count_walls(&level_single);
        
        // 3D generation should place fewer walls (be less aggressive)
        assert!(walls_3d <= walls_single, 
               "3D generation should place fewer walls: {} vs {}", walls_3d, walls_single);
    }

    #[test]
    fn test_generate_complete_dungeon_performance() {
        let generator = RoomCorridorGenerator::new();
        let config = GenerationConfig::for_testing(77777);
        let mut rng = utils::create_rng(&config);
        
        let start_time = std::time::Instant::now();
        let world = generator.generate_complete_dungeon(&config, &mut rng).unwrap();
        let generation_time = start_time.elapsed();
        
        // Should complete in reasonable time (less than 30 seconds in debug mode)
        assert!(generation_time.as_secs() < 30, 
               "Generation took too long: {:?}", generation_time);
        
        // Verify world integrity
        assert_eq!(world.levels.len(), 26);
        assert_eq!(world.current_level_id, 0);
        
        // Each level should be valid
        for level_id in 0..26 {
            let level = world.get_level(level_id).unwrap();
            assert!(utils::validate_level(level).is_ok(), 
                   "Level {} should be valid", level_id);
        }
        
        println!("Generated complete 26-level dungeon in {:?}", generation_time);
    }

    #[test]
    fn test_3d_generation_stair_connectivity() {
        let generator = RoomCorridorGenerator::new();
        let config = GenerationConfig::for_testing(88888);
        let mut rng = utils::create_rng(&config);
        
        let world = generator.generate_complete_dungeon(&config, &mut rng).unwrap();
        
        // Test that stairs are connected within each level
        for level_id in 1..25 { // Skip level 0 (no up stairs) and 25 (no down stairs)
            let level = world.get_level(level_id).unwrap();
            
            if let (Some(up_pos), Some(down_pos)) = (level.stairs_up_position, level.stairs_down_position) {
                // There should be a path between up and down stairs
                assert!(generator.has_path(level, up_pos, down_pos).unwrap(),
                       "Stairs should be connected on level {}", level_id);
            }
        }
    }

    #[test]
    fn test_world_generator_error_handling() {
        let generator = RoomCorridorGenerator::new();
        
        // Test with invalid config (this shouldn't fail but let's test the pipeline)
        let config = GenerationConfig::for_testing(99999);
        let mut rng = utils::create_rng(&config);
        
        let world = generator.generate_world(&config, &mut rng);
        assert!(world.is_ok(), "World generation should handle edge cases gracefully");
        
        if let Ok(world) = world {
            let validation = generator.validate_world(&world, &config);
            assert!(validation.is_ok(), "Generated world should pass validation");
        }
    }
}
