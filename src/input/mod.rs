//! # Input Module
//!
//! Input handling and command parsing for player interactions.

pub mod commands;

pub use commands::*;

use crate::{ThatchError, ThatchResult};
use crate::game::{
    ConcreteAction, Direction, Entity, GameState, MoveAction, Position, 
    StairDirection, UseStairsAction, WaitAction,
};
use macroquad::prelude::*;

/// Input handler for processing player commands.
///
/// Handles keyboard input and converts it to game actions that can be
/// processed by the game state.
pub struct InputHandler {
    /// Whether to enable Vi-style movement keys (hjkl)
    pub vi_keys_enabled: bool,
}

impl Default for InputHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl InputHandler {
    /// Creates a new input handler.
    ///
    /// # Examples
    ///
    /// ```
    /// use thatch::InputHandler;
    ///
    /// let input_handler = InputHandler::new();
    /// // Ready to process input
    /// ```
    pub fn new() -> Self {
        Self {
            vi_keys_enabled: true,
        }
    }

    /// Gets the current input if any key is pressed.
    ///
    /// Returns the corresponding player input, or None if no key is pressed.
    pub fn get_input(&self) -> Option<PlayerInput> {
        self.process_macroquad_input()
    }

    /// Processes macroquad input and returns the corresponding player input.
    fn process_macroquad_input(&self) -> Option<PlayerInput> {
        // Check for quit
        if is_key_pressed(KeyCode::Escape) {
            return Some(PlayerInput::Quit);
        }

        // Movement keys - Arrow keys
        if is_key_pressed(KeyCode::Up) {
            return Some(PlayerInput::Move(Position::new(0, -1)));
        }
        if is_key_pressed(KeyCode::Down) {
            return Some(PlayerInput::Move(Position::new(0, 1)));
        }
        if is_key_pressed(KeyCode::Left) {
            return Some(PlayerInput::Move(Position::new(-1, 0)));
        }
        if is_key_pressed(KeyCode::Right) {
            return Some(PlayerInput::Move(Position::new(1, 0)));
        }

        // Movement keys - WASD
        if is_key_pressed(KeyCode::W) {
            return Some(PlayerInput::Move(Position::new(0, -1)));
        }
        if is_key_pressed(KeyCode::S) {
            return Some(PlayerInput::Move(Position::new(0, 1)));
        }
        if is_key_pressed(KeyCode::A) {
            return Some(PlayerInput::Move(Position::new(-1, 0)));
        }
        if is_key_pressed(KeyCode::D) {
            return Some(PlayerInput::Move(Position::new(1, 0)));
        }

        // Movement keys - Vi style (hjkl) if enabled
        if self.vi_keys_enabled {
            if is_key_pressed(KeyCode::H) {
                return Some(PlayerInput::Move(Position::new(-1, 0)));
            }
            if is_key_pressed(KeyCode::J) {
                return Some(PlayerInput::Move(Position::new(0, 1)));
            }
            if is_key_pressed(KeyCode::K) {
                return Some(PlayerInput::Move(Position::new(0, -1)));
            }
            if is_key_pressed(KeyCode::L) {
                return Some(PlayerInput::Move(Position::new(1, 0)));
            }

            // Diagonal movement
            if is_key_pressed(KeyCode::Y) {
                return Some(PlayerInput::Move(Position::new(-1, -1)));
            }
            if is_key_pressed(KeyCode::U) {
                return Some(PlayerInput::Move(Position::new(1, -1)));
            }
            if is_key_pressed(KeyCode::B) {
                return Some(PlayerInput::Move(Position::new(-1, 1)));
            }
            if is_key_pressed(KeyCode::N) {
                return Some(PlayerInput::Move(Position::new(1, 1)));
            }
        }

        // Wait/rest
        if is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::Period) {
            return Some(PlayerInput::Wait);
        }

        // Help
        if is_key_pressed(KeyCode::F1) {
            return Some(PlayerInput::Help);
        }

        // Inventory
        if is_key_pressed(KeyCode::I) {
            return Some(PlayerInput::ShowInventory);
        }

        // Pick up item
        if is_key_pressed(KeyCode::Comma) || is_key_pressed(KeyCode::G) {
            return Some(PlayerInput::PickUp);
        }

        // Enter (confirm action)
        if is_key_pressed(KeyCode::Enter) {
            return Some(PlayerInput::Confirm);
        }

        // Stairs - using number keys since < > are hard to press
        if is_key_pressed(KeyCode::Key1) {
            return Some(PlayerInput::UseStairs(StairDirection::Up));
        }
        if is_key_pressed(KeyCode::Key2) {
            return Some(PlayerInput::UseStairs(StairDirection::Down));
        }

        None
    }

    /// Converts player input to a concrete game action.
    ///
    /// This takes the player input and the current game state to determine
    /// what action should be executed.
    pub fn input_to_action(
        &self,
        input: PlayerInput,
        game_state: &GameState,
    ) -> ThatchResult<Option<ConcreteAction>> {
        match input {
            PlayerInput::Move(delta) => {
                if let Some(player) = game_state.get_player() {
                    if let Some(direction) = Direction::from_delta(delta) {
                        Ok(Some(ConcreteAction::Move(MoveAction {
                            actor: player.id(),
                            direction,
                            metadata: std::collections::HashMap::new(),
                        })))
                    } else {
                        Err(ThatchError::InvalidAction(
                            "Invalid movement direction".to_string(),
                        ))
                    }
                } else {
                    Err(ThatchError::InvalidState("No player found".to_string()))
                }
            }

            PlayerInput::Wait => {
                if let Some(player) = game_state.get_player() {
                    Ok(Some(ConcreteAction::Wait(WaitAction {
                        actor: player.id(),
                        metadata: std::collections::HashMap::new(),
                    })))
                } else {
                    Err(ThatchError::InvalidState("No player found".to_string()))
                }
            }

            PlayerInput::UseStairs(direction) => {
                if let Some(player) = game_state.get_player() {
                    Ok(Some(ConcreteAction::UseStairs(UseStairsAction::new(
                        player.id(),
                        direction,
                    ))))
                } else {
                    Err(ThatchError::InvalidState("No player found".to_string()))
                }
            }

            // Other inputs don't translate directly to game actions
            _ => Ok(None),
        }
    }
}

/// Player input types that can be processed by the input handler.
#[derive(Debug, Clone, PartialEq)]
pub enum PlayerInput {
    /// Move in a given direction (relative position)
    Move(Position),
    /// Wait/rest for one turn
    Wait,
    /// Quit the game
    Quit,
    /// Show help information
    Help,
    /// Show inventory
    ShowInventory,
    /// Pick up item at current position
    PickUp,
    /// Cancel current action
    Cancel,
    /// Confirm current action
    Confirm,
    /// Use stairs in the specified direction
    UseStairs(StairDirection),
    /// Start a new game (when game has ended)
    NewGame,
}
