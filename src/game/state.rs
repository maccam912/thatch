//! # Game State Module
//!
//! Central game state management and coordination between all game systems.
//!
//! This module contains the main GameState struct that coordinates all aspects
//! of the game world, entities, and systems. It provides the primary interface
//! for game operations and maintains consistency across all game components.

use crate::{
    ActionQueue, AutoexploreState, ConcreteEntity, Direction, Entity, EntityId, EntityStats,
    GameEvent, Level, MoveAction, PlayerCharacter, Position, StairDirection, ThatchError,
    ThatchResult, TileType, UseStairsAction, World,
};
use serde::{Deserialize, Serialize};
use std::collections::{BinaryHeap, HashMap};
use std::time::{Duration, Instant};

/// Central game state containing all game data and systems.
///
/// This is the main coordination point for all game operations. It maintains
/// the game world, entities, turn management, and provides interfaces for
/// both player actions and MCP integration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    /// The game world containing all levels
    pub world: World,
    /// All entities in the game, indexed by ID
    pub entities: HashMap<EntityId, ConcreteEntity>,
    /// Spatial index mapping positions to entity IDs
    pub position_index: HashMap<Position, Vec<EntityId>>,
    /// The player entity ID
    pub player_id: Option<EntityId>,
    /// Action queue for turn management
    pub action_queue: ActionQueue,
    /// Current game turn number
    pub turn_number: u64,
    /// Game start time
    #[serde(skip)]
    pub game_start_time: Option<Instant>,
    /// Total play time in seconds
    pub total_play_time: u64,
    /// Game configuration flags
    pub config_flags: HashMap<String, bool>,
    /// Game statistics for player progress
    pub statistics: GameStatistics,
    /// Random number generator seed
    pub rng_seed: u64,
    /// LLDM integration state
    pub lldm_state: LldmState,
    /// Current game completion state
    pub completion_state: GameCompletionState,
    /// Autoexplore debug state (not serialized)
    #[serde(skip)]
    pub autoexplore_state: AutoexploreState,
}

/// Game statistics tracking player progress and achievements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameStatistics {
    /// Number of enemies defeated
    pub enemies_defeated: u32,
    /// Number of levels explored
    pub levels_explored: u32,
    /// Number of items collected
    pub items_collected: u32,
    /// Total damage dealt
    pub damage_dealt: u64,
    /// Total damage taken
    pub damage_taken: u64,
    /// Number of times the player has died
    pub deaths: u32,
    /// Deepest level reached
    pub max_depth_reached: u32,
    /// Total steps taken
    pub steps_taken: u64,
    /// Rooms discovered
    pub rooms_discovered: u32,
    /// Secrets found
    pub secrets_found: u32,
}

impl GameStatistics {
    /// Creates new empty statistics.
    pub fn new() -> Self {
        Self {
            enemies_defeated: 0,
            levels_explored: 0,
            items_collected: 0,
            damage_dealt: 0,
            damage_taken: 0,
            deaths: 0,
            max_depth_reached: 0,
            steps_taken: 0,
            rooms_discovered: 0,
            secrets_found: 0,
        }
    }

    /// Updates statistics based on a game event.
    pub fn update_from_event(&mut self, event: &GameEvent) {
        match event {
            GameEvent::EntityMoved { .. } => {
                self.steps_taken += 1;
            }
            GameEvent::EntityDamaged { damage, .. } => {
                self.damage_dealt += *damage as u64;
            }
            GameEvent::EntityDied { killer, .. } => {
                if killer.is_some() {
                    self.enemies_defeated += 1;
                }
            }
            GameEvent::ItemPickedUp { .. } => {
                self.items_collected += 1;
            }
            _ => {}
        }
    }
}

impl Default for GameStatistics {
    fn default() -> Self {
        Self::new()
    }
}

/// Game completion state for handling endings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameCompletionState {
    /// Game is still in progress
    Playing,
    /// Player escaped from level 1 (medium ending)
    EscapedEarly,
    /// Player reached bottom level and won (good ending)
    CompletedDungeon,
    /// Player died
    PlayerDied,
}

/// State for LLDM (LLM Dungeon Master) integration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LldmState {
    /// Whether LLDM is enabled
    pub enabled: bool,
    /// Current LLDM session ID
    pub session_id: Option<String>,
    /// LLDM-generated content cache
    pub content_cache: HashMap<String, String>,
    /// Pending LLDM requests
    pub pending_requests: Vec<LldmRequest>,
    /// LLDM configuration
    pub config: LldmConfig,
}

/// Configuration for LLDM integration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LldmConfig {
    /// API endpoint for LLDM
    pub endpoint: Option<String>,
    /// Model to use for generation
    pub model: String,
    /// Temperature for generation
    pub temperature: f32,
    /// Maximum tokens per request
    pub max_tokens: u32,
    /// Whether to use cached responses
    pub use_cache: bool,
}

/// Request to the LLDM system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LldmRequest {
    /// Request ID for tracking
    pub id: String,
    /// Type of content to generate
    pub request_type: String,
    /// Context for generation
    pub context: HashMap<String, String>,
    /// Whether this is urgent
    pub priority: LldmPriority,
    /// Timestamp when request was created
    pub created_at: u64,
}

/// Priority levels for LLDM requests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LldmPriority {
    Low,
    Normal,
    High,
    Urgent,
}

impl GameState {
    /// Creates a new game state with default world.
    ///
    /// # Examples
    ///
    /// ```
    /// use thatch::GameState;
    ///
    /// let game_state = GameState::new(12345);
    /// assert_eq!(game_state.turn_number, 0);
    /// assert!(game_state.player_id.is_none());
    /// ```
    pub fn new(seed: u64) -> Self {
        Self {
            world: World::new(seed),
            entities: HashMap::new(),
            position_index: HashMap::new(),
            player_id: None,
            action_queue: ActionQueue::new(),
            turn_number: 0,
            game_start_time: None,
            total_play_time: 0,
            config_flags: HashMap::new(),
            statistics: GameStatistics::new(),
            rng_seed: seed,
            lldm_state: LldmState {
                enabled: false,
                session_id: None,
                content_cache: HashMap::new(),
                pending_requests: Vec::new(),
                config: LldmConfig {
                    endpoint: None,
                    model: "gpt-4".to_string(),
                    temperature: 0.7,
                    max_tokens: 1000,
                    use_cache: true,
                },
            },
            completion_state: GameCompletionState::Playing,
            autoexplore_state: AutoexploreState::new(),
        }
    }

    /// Creates a new game state with a complete 3D dungeon pre-generated.
    ///
    /// This method generates all 26 floors at once with proper stair alignment,
    /// which is more efficient and ensures consistency across levels.
    pub fn new_with_complete_dungeon(seed: u64) -> ThatchResult<Self> {
        use crate::{GenerationConfig, RoomCorridorGenerator, WorldGenerator};
        use rand::{rngs::StdRng, SeedableRng};

        let config = GenerationConfig::new(seed);
        let mut rng = StdRng::seed_from_u64(seed);
        let generator = RoomCorridorGenerator::new();

        // Generate complete 3D dungeon
        let world = generator.generate_world(&config, &mut rng)?;

        Ok(Self {
            world,
            entities: HashMap::new(),
            position_index: HashMap::new(),
            player_id: None,
            action_queue: ActionQueue::new(),
            turn_number: 0,
            game_start_time: None,
            total_play_time: 0,
            config_flags: HashMap::new(),
            statistics: GameStatistics::new(),
            rng_seed: seed,
            lldm_state: LldmState {
                enabled: false,
                session_id: None,
                content_cache: HashMap::new(),
                pending_requests: Vec::new(),
                config: LldmConfig {
                    endpoint: None,
                    model: "gpt-4".to_string(),
                    temperature: 0.7,
                    max_tokens: 1000,
                    use_cache: true,
                },
            },
            completion_state: GameCompletionState::Playing,
            autoexplore_state: AutoexploreState::new(),
        })
    }

    /// Initializes the game with a player character.
    ///
    /// # Examples
    ///
    /// ```
    /// use thatch::{GameState, Position};
    ///
    /// let mut game_state = GameState::new(12345);
    /// let player_id = game_state.initialize_player("Hero".to_string(), Position::new(5, 5)).unwrap();
    /// assert_eq!(game_state.player_id, Some(player_id));
    /// ```
    pub fn initialize_player(
        &mut self,
        name: String,
        position: Position,
    ) -> ThatchResult<EntityId> {
        // Create player character
        let player = PlayerCharacter::new(name, position);
        let player_id = player.id();

        // Add to entities
        self.entities
            .insert(player_id, ConcreteEntity::Player(player));

        // Update position index
        self.add_entity_to_position_index(player_id, position);

        // Set as player
        self.player_id = Some(player_id);

        // Add to current level
        if let Some(level) = self.world.current_level_mut() {
            level.add_entity(player_id);
        }

        // Start game timer
        self.game_start_time = Some(Instant::now());

        Ok(player_id)
    }

    /// Gets the player character if it exists.
    pub fn get_player(&self) -> Option<&PlayerCharacter> {
        if let Some(player_id) = self.player_id {
            if let Some(ConcreteEntity::Player(player)) = self.entities.get(&player_id) {
                return Some(player);
            }
        }
        None
    }

    /// Gets the player character mutably if it exists.
    pub fn get_player_mut(&mut self) -> Option<&mut PlayerCharacter> {
        if let Some(player_id) = self.player_id {
            if let Some(ConcreteEntity::Player(player)) = self.entities.get_mut(&player_id) {
                return Some(player);
            }
        }
        None
    }

    /// Creates a new game state with a specific level.
    ///
    /// This method is used when you have a pre-generated level to use
    /// instead of creating a default world.
    pub fn new_with_level(level: Level, seed: u64) -> ThatchResult<Self> {
        let mut world = World::new(seed);
        // Replace the default level (ID 0) with our generated level
        world.add_level(level); // This replaces the level with ID 0
                                // No need to change level since we're already on level 0

        Ok(Self {
            world,
            entities: HashMap::new(),
            position_index: HashMap::new(),
            player_id: None,
            action_queue: ActionQueue::new(),
            turn_number: 0,
            game_start_time: None,
            total_play_time: 0,
            config_flags: HashMap::new(),
            statistics: GameStatistics::new(),
            rng_seed: seed,
            lldm_state: LldmState {
                enabled: false,
                session_id: None,
                content_cache: HashMap::new(),
                pending_requests: Vec::new(),
                config: LldmConfig {
                    endpoint: None,
                    model: "gpt-4".to_string(),
                    temperature: 0.7,
                    max_tokens: 1000,
                    use_cache: true,
                },
            },
            completion_state: GameCompletionState::Playing,
            autoexplore_state: AutoexploreState::new(),
        })
    }

    /// Finds a suitable starting position for the player.
    ///
    /// Searches the current level for a floor tile that's not occupied
    /// by another entity. Prefers positions in rooms over corridors.
    pub fn find_starting_position(&self) -> ThatchResult<Position> {
        let level = self
            .world
            .current_level()
            .ok_or_else(|| ThatchError::InvalidState("No current level".to_string()))?;

        // First try to find a position in a room
        for y in 1..level.height - 1 {
            for x in 1..level.width - 1 {
                let pos = Position::new(x as i32, y as i32);
                if let Some(tile) = level.get_tile(pos) {
                    if tile.tile_type == TileType::Floor
                        && self.get_entities_at_position(pos).is_empty()
                    {
                        return Ok(pos);
                    }
                }
            }
        }

        Err(ThatchError::InvalidState(
            "No suitable starting position found".to_string(),
        ))
    }

    /// Adds an entity to the game state.
    ///
    /// Returns the entity ID for future reference.
    pub fn add_entity(&mut self, entity: ConcreteEntity) -> ThatchResult<EntityId> {
        let entity_id = entity.id();
        let position = entity.position();

        self.entities.insert(entity_id, entity);
        self.add_entity_to_position_index(entity_id, position);

        Ok(entity_id)
    }

    /// Sets the player entity ID.
    ///
    /// This should be called after adding the player entity to track
    /// which entity is controlled by the player.
    pub fn set_player_id(&mut self, player_id: EntityId) {
        self.player_id = Some(player_id);
    }

    /// Checks if an entity exists.
    pub fn entity_exists(&self, entity_id: EntityId) -> bool {
        self.entities.contains_key(&entity_id)
    }

    /// Checks if an entity is alive.
    pub fn is_entity_alive(&self, entity_id: EntityId) -> bool {
        self.entities
            .get(&entity_id)
            .map(|entity| entity.is_alive())
            .unwrap_or(false)
    }

    /// Gets an entity's position.
    pub fn get_entity_position(&self, entity_id: EntityId) -> Option<Position> {
        self.entities
            .get(&entity_id)
            .map(|entity| entity.position())
    }

    /// Sets an entity's position.
    pub fn set_entity_position(
        &mut self,
        entity_id: EntityId,
        new_position: Position,
    ) -> ThatchResult<()> {
        // Get old position first
        let old_position = self
            .get_entity_position(entity_id)
            .ok_or_else(|| ThatchError::InvalidState("Entity not found".to_string()))?;

        // Remove from old position in index
        self.remove_entity_from_position_index(entity_id, old_position);

        // Update entity position
        match self.entities.get_mut(&entity_id) {
            Some(ConcreteEntity::Player(player)) => {
                player.set_position(new_position);
            }
            None => {
                return Err(ThatchError::InvalidState(format!(
                    "Entity {} not found for position update",
                    entity_id
                )));
            }
        }

        // Add to new position in index
        self.add_entity_to_position_index(entity_id, new_position);

        Ok(())
    }

    /// Gets an entity at a specific position.
    pub fn get_entity_at_position(&self, position: Position) -> Option<EntityId> {
        self.position_index
            .get(&position)
            .and_then(|entities| entities.first().copied())
    }

    /// Gets all entities at a specific position.
    pub fn get_entities_at_position(&self, position: Position) -> Vec<EntityId> {
        self.position_index
            .get(&position)
            .cloned()
            .unwrap_or_default()
    }

    /// Gets entity stats (if applicable).
    pub fn get_entity_stats(&self, entity_id: EntityId) -> Option<&EntityStats> {
        match self.entities.get(&entity_id) {
            Some(ConcreteEntity::Player(player)) => Some(&player.stats),
            None => None,
        }
    }

    /// Processes a game event and updates state accordingly.
    pub fn process_event(&mut self, event: &GameEvent) -> ThatchResult<Vec<GameEvent>> {
        let mut response_events = Vec::new();

        // Update statistics
        self.statistics.update_from_event(event);

        // Handle event-specific processing
        match event {
            GameEvent::EntityMoved {
                entity_id,
                from: _,
                to,
            } => {
                // Position index is already updated by set_entity_position
                // Update visibility if this is the player
                if Some(*entity_id) == self.player_id {
                    self.update_player_visibility(*to)?;
                }
            }

            GameEvent::EntityDamaged {
                entity_id,
                damage: _,
                source: _,
            } => {
                // Forward to the entity for handling
                if let Some(entity) = self.entities.get_mut(entity_id) {
                    match entity {
                        ConcreteEntity::Player(player) => {
                            let events = player.handle_event(event)?;
                            response_events.extend(events);
                        }
                    }
                }
            }

            GameEvent::EntityDied { entity_id, .. } => {
                // Remove entity from world
                if let Some(position) = self.get_entity_position(*entity_id) {
                    self.remove_entity_from_position_index(*entity_id, position);
                }

                // Remove from current level
                if let Some(level) = self.world.current_level_mut() {
                    level.remove_entity(entity_id);
                }

                // If this is the player, handle game over
                if Some(*entity_id) == self.player_id {
                    self.statistics.deaths += 1;
                    response_events.push(GameEvent::Message {
                        text: "Game Over! Press any key to continue...".to_string(),
                        importance: crate::MessageImportance::Critical,
                    });
                }
            }

            _ => {}
        }

        Ok(response_events)
    }

    /// Updates player's field of view and tile visibility.
    /// This preserves exploration state while updating current visibility.
    pub fn update_player_visibility(&mut self, player_position: Position) -> ThatchResult<()> {
        let player = self
            .get_player()
            .ok_or_else(|| ThatchError::InvalidState("No player found".to_string()))?;

        let sight_radius = player.sight_radius as i32;

        // Simple visibility algorithm (can be improved with line-of-sight)
        let level = self
            .world
            .current_level_mut()
            .ok_or_else(|| ThatchError::InvalidState("No current level".to_string()))?;

        // Reset all tiles to not visible (but preserve exploration state)
        for row in &mut level.tiles {
            for tile in row {
                tile.visible = false; // Don't use set_visible as it would mark as explored
            }
        }

        // Set visible tiles within sight radius
        for dy in -sight_radius..=sight_radius {
            for dx in -sight_radius..=sight_radius {
                let pos = Position::new(player_position.x + dx, player_position.y + dy);

                // Check if position is within sight radius (circular)
                if player_position.euclidean_distance(pos) <= sight_radius as f64 {
                    if let Some(tile) = level.get_tile_mut(pos) {
                        tile.set_visible(true); // This marks as explored and visible
                    }
                }
            }
        }

        Ok(())
    }

    /// Advances the game by one turn.
    pub fn advance_turn(&mut self) -> ThatchResult<Vec<GameEvent>> {
        self.turn_number += 1;

        // Update total play time
        if let Some(start_time) = self.game_start_time {
            self.total_play_time = start_time.elapsed().as_secs();
        }

        // Process any pending LLDM requests
        self.process_lldm_requests()?;

        // Additional turn processing can be added here
        Ok(vec![])
    }

    /// Gets current game time information.
    pub fn get_game_time_info(&self) -> GameTimeInfo {
        let elapsed = self
            .game_start_time
            .map(|start| start.elapsed())
            .unwrap_or(Duration::ZERO);

        GameTimeInfo {
            turn_number: self.turn_number,
            elapsed_time: elapsed,
            total_play_time: Duration::from_secs(self.total_play_time),
        }
    }

    /// Gets configuration flag value.
    pub fn get_config_flag(&self, flag: &str) -> bool {
        self.config_flags.get(flag).copied().unwrap_or(false)
    }

    /// Sets configuration flag value.
    pub fn set_config_flag(&mut self, flag: String, value: bool) {
        self.config_flags.insert(flag, value);
    }

    /// Adds entity to position index.
    fn add_entity_to_position_index(&mut self, entity_id: EntityId, position: Position) {
        self.position_index
            .entry(position)
            .or_default()
            .push(entity_id);
    }

    /// Removes entity from position index.
    fn remove_entity_from_position_index(&mut self, entity_id: EntityId, position: Position) {
        if let Some(entities) = self.position_index.get_mut(&position) {
            entities.retain(|&id| id != entity_id);
            if entities.is_empty() {
                self.position_index.remove(&position);
            }
        }
    }

    /// Processes pending LLDM requests.
    fn process_lldm_requests(&mut self) -> ThatchResult<()> {
        if !self.lldm_state.enabled {
            return Ok(());
        }

        // In a full implementation, this would make actual API calls
        // For now, we just clear processed requests
        self.lldm_state.pending_requests.clear();

        Ok(())
    }

    /// Saves the game state to JSON.
    pub fn save_to_json(&self) -> ThatchResult<String> {
        serde_json::to_string_pretty(self).map_err(ThatchError::from)
    }

    /// Loads game state from JSON.
    pub fn load_from_json(json: &str) -> ThatchResult<Self> {
        serde_json::from_str(json).map_err(ThatchError::from)
    }

    /// Handles level progression when player uses stairs.
    ///
    /// Returns true if the level change was successful, false if it triggers a game ending.
    pub fn use_stairs(&mut self, direction: crate::StairDirection) -> ThatchResult<bool> {
        let current_level_id = self.world.current_level_id;

        match direction {
            crate::StairDirection::Up => {
                if current_level_id == 0 {
                    // Going up from level 1 triggers escape ending
                    self.completion_state = GameCompletionState::EscapedEarly;
                    return Ok(false);
                } else {
                    // Go back to previous level
                    let target_level_id = current_level_id - 1;
                    self.change_to_level(target_level_id)?;
                }
            }
            crate::StairDirection::Down => {
                if current_level_id >= 26 {
                    // Going down from level 27 (0-indexed 26) triggers win ending
                    self.completion_state = GameCompletionState::CompletedDungeon;
                    return Ok(false);
                } else {
                    // Go to next level (generate if needed)
                    let target_level_id = current_level_id + 1;
                    self.change_to_level(target_level_id)?;
                }
            }
        }

        Ok(true)
    }

    /// Changes to the specified level, generating it if it doesn't exist.
    fn change_to_level(&mut self, level_id: u32) -> ThatchResult<()> {
        // If level doesn't exist, generate it
        if !self.world.levels.contains_key(&level_id) {
            // For the new 3D generation system, all levels should already exist
            // Only generate on-demand if using the old system
            if self.world.levels.len() == 1 {
                // Old system: only has 1 level initially, generate more as needed
                self.generate_level(level_id)?;
            } else {
                // New 3D system: all levels should already exist
                return Err(ThatchError::InvalidState(format!(
                    "Level {} does not exist in pre-generated world",
                    level_id
                )));
            }
        }

        // Move player entity from current level to target level
        if let Some(player_id) = self.player_id {
            // Remove from current level
            if let Some(current_level) = self.world.current_level_mut() {
                current_level.remove_entity(&player_id);
            }

            // Change level
            self.world.change_level(level_id)?;

            // Add to new level and move to spawn point (stairs)
            if let Some(new_level) = self.world.current_level_mut() {
                new_level.add_entity(player_id);
                let spawn_pos = new_level.player_spawn; // This is now always stairs up

                // Update entity position
                let old_pos = if let Some(player) = self.get_player() {
                    player.position()
                } else {
                    spawn_pos // fallback
                };

                self.remove_entity_from_position_index(player_id, old_pos);
                if let Some(player) = self.get_player_mut() {
                    player.set_position(spawn_pos);
                }
                self.add_entity_to_position_index(player_id, spawn_pos);
            }

            // CRITICAL: Update visibility immediately after level change
            // This ensures the player can see around them when entering a level
            if let Some(player_pos) = self.get_entity_position(player_id) {
                self.update_player_visibility(player_pos)?;
            }

            // Update statistics
            if level_id > self.statistics.max_depth_reached {
                self.statistics.max_depth_reached = level_id;
                self.statistics.levels_explored += 1;
            }

            // Force an immediate visibility update to prevent "blank screen" bug
            if let Some(player_pos) = self.get_entity_position(player_id) {
                let _ = self.update_player_visibility(player_pos);
            }
        }

        Ok(())
    }

    /// Generates a new level with the specified ID.
    fn generate_level(&mut self, level_id: u32) -> ThatchResult<()> {
        use crate::{GenerationConfig, Generator, RoomCorridorGenerator};
        use rand::{rngs::StdRng, SeedableRng};

        // Create level-specific seed based on world seed and level ID
        let level_seed = self.rng_seed.wrapping_add(level_id as u64 * 1000);
        let mut rng = StdRng::seed_from_u64(level_seed);

        let config = GenerationConfig::default();
        let generator = RoomCorridorGenerator::new();

        let mut level = generator.generate(&config, &mut rng)?;
        level.id = level_id;

        // Set level name based on depth
        level.name = Some(format!("Dungeon Level {}", level_id + 1));

        // Align stairs with previous level if possible
        self.align_stairs_with_previous_level(&mut level, level_id);

        self.world.add_level(level);
        Ok(())
    }

    /// Aligns stairs between levels for consistent navigation.
    fn align_stairs_with_previous_level(&self, level: &mut Level, level_id: u32) {
        // If going down from previous level, align stairs up with previous level's stairs down
        if level_id > 0 {
            if let Some(prev_level) = self.world.get_level(level_id - 1) {
                if let Some(prev_stairs_down) = prev_level.stairs_down_position {
                    // Try to place stairs up at the same position as previous level's stairs down
                    if level.is_valid_position(prev_stairs_down) {
                        // Make sure the position is or can be made passable
                        let _ = level.set_tile(
                            prev_stairs_down,
                            crate::Tile::new(crate::TileType::StairsUp),
                        );
                        level.stairs_up_position = Some(prev_stairs_down);
                        level.player_spawn = prev_stairs_down;

                        // Ensure there's a clear area around the stairs
                        self.clear_area_around_stairs(level, prev_stairs_down);
                    }
                }
            }
        }

        // If going up to next level, try to align stairs down for future consistency
        // This is handled when the next level is generated
    }

    /// Clears a small area around stairs to ensure accessibility.
    fn clear_area_around_stairs(&self, level: &mut Level, stairs_pos: Position) {
        // Clear a 3x3 area around stairs to ensure accessibility
        for dy in -1..=1 {
            for dx in -1..=1 {
                let clear_pos = Position::new(stairs_pos.x + dx, stairs_pos.y + dy);
                if level.is_valid_position(clear_pos) && clear_pos != stairs_pos {
                    // Only clear if it's not a boundary wall
                    if clear_pos.x > 0
                        && clear_pos.y > 0
                        && clear_pos.x < (level.width as i32 - 1)
                        && clear_pos.y < (level.height as i32 - 1)
                    {
                        let _ = level.set_tile(clear_pos, crate::Tile::floor());
                    }
                }
            }
        }
    }

    /// Resets the game state for a new game.
    pub fn reset_for_new_game(&mut self) -> ThatchResult<()> {
        // Clear all levels except level 0
        self.world.levels.retain(|&id, _| id == 0);
        self.world.current_level_id = 0;
        self.world.max_depth = 0;

        // Regenerate level 0
        self.generate_level(0)?;

        // Reset player position to spawn
        if let Some(player_id) = self.player_id {
            let spawn_pos = if let Some(level) = self.world.current_level() {
                level.player_spawn
            } else {
                Position::new(0, 0)
            };

            let old_pos = if let Some(player) = self.get_player() {
                player.position()
            } else {
                spawn_pos // fallback
            };

            self.remove_entity_from_position_index(player_id, old_pos);
            if let Some(player) = self.get_player_mut() {
                player.set_position(spawn_pos);
            }
            self.add_entity_to_position_index(player_id, spawn_pos);
        }

        // Reset game state
        self.completion_state = GameCompletionState::Playing;
        self.turn_number = 0;
        self.statistics = GameStatistics::new();
        self.game_start_time = Some(Instant::now());

        Ok(())
    }

    /// Checks if the game has ended.
    pub fn is_game_ended(&self) -> bool {
        self.completion_state != GameCompletionState::Playing
    }

    /// Gets the current completion state.
    pub fn get_completion_state(&self) -> &GameCompletionState {
        &self.completion_state
    }

    /// Toggles autoexplore debug mode.
    pub fn toggle_autoexplore(&mut self) -> bool {
        self.autoexplore_state.toggle()
    }

    /// Gets the next autoexplore action if enabled and ready.
    pub fn get_autoexplore_action(&mut self) -> ThatchResult<Option<crate::ConcreteAction>> {
        if !self.autoexplore_state.enabled || !self.autoexplore_state.can_perform_action() {
            return Ok(None);
        }

        let player = self
            .get_player()
            .ok_or_else(|| ThatchError::InvalidState("No player found".to_string()))?;
        let player_pos = player.position();
        let player_id = player.id();

        // Check if we're already on stairs down
        if let Some(level) = self.world.current_level() {
            if let Some(tile) = level.get_tile(player_pos) {
                if tile.tile_type == TileType::StairsDown {
                    // We're on stairs down, use them
                    self.autoexplore_state.mark_action_performed();
                    return Ok(Some(crate::ConcreteAction::UseStairs(
                        UseStairsAction::new(player_id, StairDirection::Down),
                    )));
                }
            }
        }

        // If we have a current path, follow it
        if !self.autoexplore_state.current_path.is_empty() {
            let next_pos = self.autoexplore_state.current_path.remove(0);
            if let Some(direction) = self.get_direction_to_position(player_pos, next_pos) {
                self.autoexplore_state.mark_action_performed();
                return Ok(Some(crate::ConcreteAction::Move(MoveAction {
                    actor: player_id,
                    direction,
                    metadata: HashMap::new(),
                })));
            } else {
                // Path is invalid, clear it
                self.autoexplore_state.current_path.clear();
            }
        }

        // We need a new path - find stairs down
        if let Some(stairs_down_pos) = self.find_stairs_down() {
            if let Some(path) = self.autoexplore_find_path(player_pos, stairs_down_pos)? {
                self.autoexplore_state.current_path = path;
                self.autoexplore_state.target = Some(stairs_down_pos);

                // Return the first move in the path
                if !self.autoexplore_state.current_path.is_empty() {
                    let next_pos = self.autoexplore_state.current_path.remove(0);
                    if let Some(direction) = self.get_direction_to_position(player_pos, next_pos) {
                        self.autoexplore_state.mark_action_performed();
                        return Ok(Some(crate::ConcreteAction::Move(MoveAction {
                            actor: player_id,
                            direction,
                            metadata: HashMap::new(),
                        })));
                    }
                }
            }
        }

        // No stairs down found or no path available
        Ok(None)
    }

    /// Helper method to get direction between positions for autoexplore.
    fn get_direction_to_position(&self, from: Position, to: Position) -> Option<Direction> {
        let delta = to - from;
        Direction::from_delta(delta)
    }

    /// Helper method to find stairs down position for autoexplore.
    fn find_stairs_down(&self) -> Option<Position> {
        let level = self.world.current_level()?;
        level.stairs_down_position
    }

    /// Helper method for autoexplore pathfinding.
    fn autoexplore_find_path(
        &self,
        start: Position,
        goal: Position,
    ) -> ThatchResult<Option<Vec<Position>>> {
        let level = self
            .world
            .current_level()
            .ok_or_else(|| ThatchError::InvalidState("No current level".to_string()))?;

        // A* algorithm implementation
        let mut open_set = BinaryHeap::new();
        let mut came_from = HashMap::new();
        let mut g_score = HashMap::new();
        let mut f_score = HashMap::new();

        g_score.insert(start, 0.0);
        f_score.insert(start, start.euclidean_distance(goal));
        open_set.push(crate::autoexplore::AStarNode {
            position: start,
            f_score: start.euclidean_distance(goal),
        });

        while let Some(current_node) = open_set.pop() {
            let current = current_node.position;

            if current == goal {
                // Reconstruct path
                let mut path = Vec::new();
                let mut current_pos = goal;

                while let Some(&prev) = came_from.get(&current_pos) {
                    path.push(current_pos);
                    current_pos = prev;
                }

                path.reverse();
                return Ok(Some(path));
            }

            // Check all adjacent positions
            for neighbor in current.adjacent_positions() {
                if !level.is_valid_position(neighbor) {
                    continue;
                }

                // Check if tile is passable
                let tile = level.get_tile(neighbor).unwrap();
                if !tile.tile_type.is_passable() {
                    continue;
                }

                // Check if there's an entity blocking the path (except at goal)
                if neighbor != goal && self.get_entity_at_position(neighbor).is_some() {
                    continue;
                }

                let tentative_g_score = g_score.get(&current).unwrap_or(&f64::INFINITY) + 1.0;

                if tentative_g_score < *g_score.get(&neighbor).unwrap_or(&f64::INFINITY) {
                    came_from.insert(neighbor, current);
                    g_score.insert(neighbor, tentative_g_score);
                    let f = tentative_g_score + neighbor.euclidean_distance(goal);
                    f_score.insert(neighbor, f);

                    // Add to open set if not already there with a better score
                    open_set.push(crate::autoexplore::AStarNode {
                        position: neighbor,
                        f_score: f,
                    });
                }
            }
        }

        Ok(None) // No path found
    }

    /// Checks if autoexplore is currently enabled.
    pub fn is_autoexplore_enabled(&self) -> bool {
        self.autoexplore_state.enabled
    }
}

/// Game time information.
#[derive(Debug, Clone)]
pub struct GameTimeInfo {
    /// Current turn number
    pub turn_number: u64,
    /// Time elapsed this session
    pub elapsed_time: Duration,
    /// Total play time across all sessions
    pub total_play_time: Duration,
}

impl Default for LldmState {
    fn default() -> Self {
        Self {
            enabled: false,
            session_id: None,
            content_cache: HashMap::new(),
            pending_requests: Vec::new(),
            config: LldmConfig {
                endpoint: None,
                model: "gpt-4".to_string(),
                temperature: 0.7,
                max_tokens: 1000,
                use_cache: true,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Position;

    #[test]
    fn test_game_state_creation() {
        let game_state = GameState::new(12345);
        assert_eq!(game_state.turn_number, 0);
        assert!(game_state.player_id.is_none());
        assert_eq!(game_state.rng_seed, 12345);
    }

    #[test]
    fn test_player_initialization() {
        let mut game_state = GameState::new(12345);
        let position = Position::new(5, 5);

        let player_id = game_state
            .initialize_player("TestHero".to_string(), position)
            .unwrap();

        assert_eq!(game_state.player_id, Some(player_id));
        assert!(game_state.entity_exists(player_id));
        assert!(game_state.is_entity_alive(player_id));
        assert_eq!(game_state.get_entity_position(player_id), Some(position));
    }

    #[test]
    fn test_entity_position_management() {
        let mut game_state = GameState::new(12345);
        let start_pos = Position::new(5, 5);
        let new_pos = Position::new(6, 6);

        let player_id = game_state
            .initialize_player("Test".to_string(), start_pos)
            .unwrap();

        // Check initial position
        assert_eq!(
            game_state.get_entity_at_position(start_pos),
            Some(player_id)
        );
        assert_eq!(game_state.get_entity_at_position(new_pos), None);

        // Move player
        game_state.set_entity_position(player_id, new_pos).unwrap();

        // Check updated positions
        assert_eq!(game_state.get_entity_at_position(start_pos), None);
        assert_eq!(game_state.get_entity_at_position(new_pos), Some(player_id));
    }

    #[test]
    fn test_turn_advancement() {
        let mut game_state = GameState::new(12345);
        assert_eq!(game_state.turn_number, 0);

        game_state.advance_turn().unwrap();
        assert_eq!(game_state.turn_number, 1);

        game_state.advance_turn().unwrap();
        assert_eq!(game_state.turn_number, 2);
    }

    #[test]
    fn test_config_flags() {
        let mut game_state = GameState::new(12345);

        assert!(!game_state.get_config_flag("debug_mode"));

        game_state.set_config_flag("debug_mode".to_string(), true);
        assert!(game_state.get_config_flag("debug_mode"));

        game_state.set_config_flag("debug_mode".to_string(), false);
        assert!(!game_state.get_config_flag("debug_mode"));
    }

    #[test]
    fn test_statistics_update() {
        let mut stats = GameStatistics::new();
        assert_eq!(stats.steps_taken, 0);
        assert_eq!(stats.damage_dealt, 0);

        let move_event = GameEvent::EntityMoved {
            entity_id: crate::new_entity_id(),
            from: Position::new(0, 0),
            to: Position::new(1, 0),
        };

        stats.update_from_event(&move_event);
        assert_eq!(stats.steps_taken, 1);

        let damage_event = GameEvent::EntityDamaged {
            entity_id: crate::new_entity_id(),
            damage: 25,
            source: None,
        };

        stats.update_from_event(&damage_event);
        assert_eq!(stats.damage_dealt, 25);
    }

    #[test]
    fn test_game_state_serialization() {
        let game_state = GameState::new(12345);
        let json = game_state.save_to_json().unwrap();

        // Should be valid JSON
        let _: serde_json::Value = serde_json::from_str(&json).unwrap();

        // Should be able to deserialize back
        let _loaded_state = GameState::load_from_json(&json).unwrap();
    }

    #[test]
    fn test_3d_dungeon_initialization() {
        let seed = 12345;
        let game_state = GameState::new_with_complete_dungeon(seed).unwrap();

        // Should have all 26 levels
        assert_eq!(game_state.world.levels.len(), 26);

        // Should start on level 0
        assert_eq!(game_state.world.current_level_id, 0);

        // Each level should be valid
        for level_id in 0..26 {
            let level = game_state.world.get_level(level_id).unwrap();
            assert_eq!(level.id, level_id);

            // Should have passable tiles
            let passable_count = level
                .tiles
                .iter()
                .flat_map(|row| row.iter())
                .filter(|tile| tile.tile_type.is_passable())
                .count();
            assert!(
                passable_count > 0,
                "Level {} should have passable tiles",
                level_id
            );
        }
    }

    #[test]
    fn test_stair_usage_level_transitions() {
        use crate::{ConcreteEntity, PlayerCharacter, StairDirection};

        let seed = 54321;
        let mut game_state = GameState::new_with_complete_dungeon(seed).unwrap();

        // Create and add player
        let player_entity = ConcreteEntity::Player(PlayerCharacter::new("TestHero".to_string()));
        let player_id = player_entity.id();
        game_state.add_entity(player_entity).unwrap();
        game_state.set_player(player_id).unwrap();

        // Start on level 0
        assert_eq!(game_state.world.current_level_id, 0);

        // Use stairs down to go to level 1
        let level_changed = game_state.use_stairs(StairDirection::Down).unwrap();
        assert!(level_changed, "Should successfully change levels");
        assert_eq!(game_state.world.current_level_id, 1);

        // Use stairs up to go back to level 0
        let level_changed = game_state.use_stairs(StairDirection::Up).unwrap();
        assert!(level_changed, "Should successfully change levels");
        assert_eq!(game_state.world.current_level_id, 0);

        // Try to go up from level 0 (should trigger escape ending)
        let level_changed = game_state.use_stairs(StairDirection::Up).unwrap();
        assert!(!level_changed, "Should not change levels - game should end");
        assert_eq!(
            game_state.completion_state,
            crate::GameCompletionState::EscapedEarly
        );
    }

    #[test]
    fn test_stair_usage_boundary_conditions() {
        use crate::{ConcreteEntity, PlayerCharacter, StairDirection};

        let seed = 98765;
        let mut game_state = GameState::new_with_complete_dungeon(seed).unwrap();

        // Create and add player
        let player_entity = ConcreteEntity::Player(PlayerCharacter::new("TestHero".to_string()));
        let player_id = player_entity.id();
        game_state.add_entity(player_entity).unwrap();
        game_state.set_player(player_id).unwrap();

        // Go to level 25
        game_state.world.change_level(25).unwrap();
        assert_eq!(game_state.world.current_level_id, 25);

        // Try to go down from level 25 (should trigger win ending)
        let level_changed = game_state.use_stairs(StairDirection::Down).unwrap();
        assert!(!level_changed, "Should not change levels - game should end");
        assert_eq!(
            game_state.completion_state,
            crate::GameCompletionState::CompletedDungeon
        );
    }

    #[test]
    fn test_change_to_level_3d_vs_single() {
        use crate::{ConcreteEntity, PlayerCharacter};

        // Test 3D system (should have all levels pre-generated)
        let seed = 11111;
        let mut game_state_3d = GameState::new_with_complete_dungeon(seed).unwrap();

        let player_entity = ConcreteEntity::Player(PlayerCharacter::new("TestHero".to_string()));
        let player_id = player_entity.id();
        game_state_3d.add_entity(player_entity).unwrap();
        game_state_3d.set_player(player_id).unwrap();

        // Should be able to change to any level 0-25
        for level_id in 0..26 {
            let result = game_state_3d.change_to_level(level_id);
            assert!(
                result.is_ok(),
                "Should be able to change to level {} in 3D system",
                level_id
            );
            assert_eq!(game_state_3d.world.current_level_id, level_id);
        }

        // Should fail for invalid levels
        assert!(game_state_3d.change_to_level(26).is_err());
        assert!(game_state_3d.change_to_level(100).is_err());

        // Test single level system (should generate on demand)
        let mut game_state_single = GameState::new(seed);
        let player_entity_2 = ConcreteEntity::Player(PlayerCharacter::new("TestHero2".to_string()));
        let player_id_2 = player_entity_2.id();
        game_state_single.add_entity(player_entity_2).unwrap();
        game_state_single.set_player(player_id_2).unwrap();

        // Should start with 1 level
        assert_eq!(game_state_single.world.levels.len(), 1);

        // Should generate level 1 on demand
        let result = game_state_single.change_to_level(1);
        assert!(result.is_ok(), "Should generate level 1 on demand");
        assert_eq!(game_state_single.world.levels.len(), 2);
    }

    #[test]
    fn test_player_position_after_level_change() {
        use crate::{ConcreteEntity, PlayerCharacter, Position};

        let seed = 22222;
        let mut game_state = GameState::new_with_complete_dungeon(seed).unwrap();

        // Create and add player
        let player_entity = ConcreteEntity::Player(PlayerCharacter::new("TestHero".to_string()));
        let player_id = player_entity.id();
        game_state.add_entity(player_entity).unwrap();
        game_state.set_player(player_id).unwrap();

        // Set initial position
        let initial_pos = Position::new(10, 10);
        game_state
            .set_entity_position(player_id, initial_pos)
            .unwrap();

        // Change to level 1
        game_state.change_to_level(1).unwrap();

        // Player should now be at spawn position of level 1 (stairs up)
        let new_pos = game_state.get_entity_position(player_id).unwrap();
        let level_1 = game_state.world.current_level().unwrap();
        assert_eq!(new_pos, level_1.player_spawn);

        // Player should be in the entities list of level 1
        assert!(level_1.entities.contains(&player_id));
    }
}
