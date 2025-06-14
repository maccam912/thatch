//! # Scene Management System
//!
//! A centralized system for managing different game scenes (playing, ending screens, etc.)
//! This eliminates the need for complex state management in the main loop.

use crate::{Entity, GameCompletionState, GameState, InputHandler, MacroquadDisplay, PlayerInput, ThatchError, ThatchResult};
use macroquad::prelude::*;

/// Represents the current scene in the game
#[derive(Debug, Clone, PartialEq)]
pub enum SceneType {
    /// Normal gameplay
    Playing,
    /// Game over screen (death, victory, or escape)
    GameOver(GameCompletionState),
}

/// The main scene manager that coordinates all game scenes
pub struct SceneManager {
    current_scene: SceneType,
    game_state: GameState,
    display: MacroquadDisplay,
    input_handler: InputHandler,
}

impl SceneManager {
    /// Creates a new scene manager with the given game state and display
    pub async fn new(game_state: GameState, input_handler: InputHandler) -> ThatchResult<Self> {
        let mut display = MacroquadDisplay::new().await?;
        display.add_message("Welcome to Thatch Roguelike!".to_string());
        display.add_message("Use WASD/arrows or touch controls to move".to_string());

        Ok(Self {
            current_scene: SceneType::Playing,
            game_state,
            display,
            input_handler,
        })
    }

    /// Runs the main scene loop until the game exits
    pub async fn run(&mut self) -> ThatchResult<()> {
        loop {
            match self.current_scene {
                SceneType::Playing => {
                    if self.update_playing_scene().await? {
                        break; // Exit requested
                    }
                }
                SceneType::GameOver(ref completion_state) => {
                    if self.update_game_over_scene(completion_state.clone()).await? {
                        break; // Exit requested
                    }
                }
            }
            next_frame().await;
        }
        Ok(())
    }

    /// Updates the playing scene, returns true if exit is requested
    async fn update_playing_scene(&mut self) -> ThatchResult<bool> {
        // Handle input
        let touch_input = self.display.get_touch_input();
        
        if let Some(input) = self.input_handler.get_input_with_touch(touch_input) {
            match input {
                PlayerInput::Quit => return Ok(true),
                
                PlayerInput::Help => {
                    self.display.add_message(
                        "Help: WASD/arrows=move, ESC=quit, SPACE=wait, F12=autoexplore, X=debug damage".to_string(),
                    );
                }

                PlayerInput::DebugDamage => {
                    self.handle_debug_damage()?;
                }

                PlayerInput::ToggleAutoexplore => {
                    let enabled = self.game_state.toggle_autoexplore();
                    if enabled {
                        self.display.add_message("Autoexplore enabled (F12 to toggle off)".to_string());
                    } else {
                        self.display.add_message("Autoexplore disabled".to_string());
                    }
                }

                _ => {
                    self.handle_game_action(input).await?;
                }
            }
        } else {
            // Handle autoexplore if no manual input
            self.handle_autoexplore().await?;
        }

        // Check for scene transition
        if self.game_state.is_game_ended() {
            self.current_scene = SceneType::GameOver(self.game_state.get_completion_state().clone());
        }

        // Render the current scene
        self.display.render_game(&self.game_state).await?;
        
        Ok(false)
    }

    /// Updates the game over scene, returns true if exit is requested
    async fn update_game_over_scene(&mut self, completion_state: GameCompletionState) -> ThatchResult<bool> {
        // Render the ending screen
        self.display.ui.render_ending_screen(&completion_state).await?;

        // Handle input
        if is_key_pressed(KeyCode::N) {
            self.start_new_game().await?;
            return Ok(false);
        } else if is_key_pressed(KeyCode::Escape) {
            return Ok(true); // Exit game
        }

        Ok(false)
    }

    /// Handles a game action (movement, etc.)
    async fn handle_game_action(&mut self, input: PlayerInput) -> ThatchResult<()> {
        if let Some(action) = self.input_handler.input_to_action(input, &self.game_state)? {
            match action.execute(&mut self.game_state) {
                Ok(events) => {
                    self.process_game_events(events).await?;
                    self.game_state.advance_turn()?;
                }
                Err(e) => {
                    // Suppress wall collision messages to reduce noise
                    if !e.to_string().contains("Position is blocked") {
                        self.display.add_message(format!("Invalid action: {}", e));
                    }
                }
            }
        }
        Ok(())
    }

    /// Handles autoexplore actions
    async fn handle_autoexplore(&mut self) -> ThatchResult<()> {
        if let Some(autoexplore_action) = self.game_state.get_autoexplore_action()? {
            match autoexplore_action.execute(&mut self.game_state) {
                Ok(events) => {
                    self.process_game_events(events).await?;
                    self.game_state.advance_turn()?;
                }
                Err(e) => {
                    // Autoexplore failed, disable it
                    self.game_state.toggle_autoexplore();
                    self.display.add_message(format!("Autoexplore disabled due to error: {}", e));
                }
            }
        }
        Ok(())
    }

    /// Processes game events and displays messages
    async fn process_game_events(&mut self, events: Vec<crate::GameEvent>) -> ThatchResult<()> {
        for event in &events {
            let response_events = self.game_state.process_event(event)?;
            
            // Display any messages from events
            for response_event in response_events {
                if let crate::GameEvent::Message { text, .. } = response_event {
                    self.display.add_message(text);
                }
            }
        }
        Ok(())
    }

    /// Handles debug damage command
    fn handle_debug_damage(&mut self) -> ThatchResult<()> {
        if let Some(player_id) = self.game_state.player_id {
            #[cfg(feature = "dev-tools")]
            tracing::info!("Debug damage command executed - dealing 150 damage");
            #[cfg(not(feature = "dev-tools"))]
            println!("Debug damage command executed - dealing 150 damage");
            
            if let Some(player) = self.game_state.get_player() {
                #[cfg(feature = "dev-tools")]
                tracing::info!("Player current health: {}/{}", player.stats.health, player.stats.max_health);
                #[cfg(not(feature = "dev-tools"))]
                println!("Player current health: {}/{}", player.stats.health, player.stats.max_health);
            }
            
            let damage_event = crate::GameEvent::EntityDamaged {
                entity_id: player_id,
                damage: 150, // Enough to kill player with 100 HP
                source: None,
            };
            
            // Process damage through the player's handle_event first
            if let Some(crate::ConcreteEntity::Player(ref mut player)) = self.game_state.entities.get_mut(&player_id) {
                #[cfg(feature = "dev-tools")]
                tracing::info!("Calling player.handle_event() directly");
                #[cfg(not(feature = "dev-tools"))]
                println!("Calling player.handle_event() directly");
                
                match player.handle_event(&damage_event) {
                    Ok(events) => {
                        #[cfg(feature = "dev-tools")]
                        tracing::info!("Player.handle_event() returned {} events", events.len());
                        #[cfg(not(feature = "dev-tools"))]
                        println!("Player.handle_event() returned {} events", events.len());
                        
                        for event in &events {
                            #[cfg(feature = "dev-tools")]
                            tracing::info!("Event from player: {:?}", event);
                            #[cfg(not(feature = "dev-tools"))]
                            println!("Event from player: {:?}", event);
                        }
                        
                        // Now process these events through game state
                        for event in events {
                            let response_events = self.game_state.process_event(&event)?;
                            for response_event in response_events {
                                if let crate::GameEvent::Message { text, .. } = response_event {
                                    self.display.add_message(text);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        #[cfg(feature = "dev-tools")]
                        tracing::error!("Error in player.handle_event(): {:?}", e);
                        #[cfg(not(feature = "dev-tools"))]
                        eprintln!("Error in player.handle_event(): {:?}", e);
                    }
                }
            }
        }
        Ok(())
    }

    /// Starts a new game with a fresh dungeon
    async fn start_new_game(&mut self) -> ThatchResult<()> {
        let new_seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        #[cfg(feature = "dev-tools")]
        tracing::info!("Starting new game with seed: {}", new_seed);
        #[cfg(not(feature = "dev-tools"))]
        println!("Starting new game with seed: {}", new_seed);

        // Create new game state
        self.game_state = GameState::new_with_complete_dungeon(new_seed)?;

        // Create and place new player
        let player_pos = if let Some(level) = self.game_state.world.current_level() {
            level.player_spawn
        } else {
            return Err(ThatchError::InvalidState("No current level".to_string()));
        };
        
        let player = crate::PlayerCharacter::new("Player".to_string(), player_pos);
        let player_id = self.game_state.add_entity(player.into())?;
        self.game_state.set_player_id(player_id);

        // Initialize player visibility
        if let Some(player) = self.game_state.get_player() {
            self.game_state.update_player_visibility(player.position())?;
        }

        // Reset scene to playing
        self.current_scene = SceneType::Playing;
        self.display.add_message("New game started!".to_string());

        Ok(())
    }
}