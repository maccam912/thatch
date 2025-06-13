//! # Game State Module
//!
//! Central game state management and coordination between all game systems.
//!
//! This module contains the main GameState struct that coordinates all aspects
//! of the game world, entities, and systems. It provides the primary interface
//! for game operations and maintains consistency across all game components.

use crate::{
    ActionQueue, ConcreteEntity, Entity, EntityId, EntityStats, GameEvent, Level,
    PlayerCharacter, Position, ThatchError, ThatchResult, TileType, World,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
        }
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

        // Reset all tiles to not visible
        for row in &mut level.tiles {
            for tile in row {
                tile.set_visible(false);
            }
        }

        // Set visible tiles within sight radius
        for dy in -sight_radius..=sight_radius {
            for dx in -sight_radius..=sight_radius {
                let pos = Position::new(player_position.x + dx, player_position.y + dy);

                // Check if position is within sight radius (circular)
                if player_position.euclidean_distance(pos) <= sight_radius as f64 {
                    if let Some(tile) = level.get_tile_mut(pos) {
                        tile.set_visible(true);
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
                if current_level_id >= 25 {
                    // Going down from level 26 (0-indexed 25) triggers win ending
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
            self.generate_level(level_id)?;
        }

        // Move player entity from current level to target level
        if let Some(player_id) = self.player_id {
            // Remove from current level
            if let Some(current_level) = self.world.current_level_mut() {
                current_level.remove_entity(&player_id);
            }

            // Change level
            self.world.change_level(level_id)?;

            // Add to new level and move to spawn point
            if let Some(new_level) = self.world.current_level_mut() {
                new_level.add_entity(player_id);
                let spawn_pos = new_level.player_spawn;
                
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

            // Update statistics
            if level_id > self.statistics.max_depth_reached {
                self.statistics.max_depth_reached = level_id;
                self.statistics.levels_explored += 1;
            }
        }

        Ok(())
    }

    /// Generates a new level with the specified ID.
    fn generate_level(&mut self, level_id: u32) -> ThatchResult<()> {
        use crate::{GenerationConfig, RoomCorridorGenerator, Generator};
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
        
        self.world.add_level(level);
        Ok(())
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
}
