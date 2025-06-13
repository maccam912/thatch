//! # Input Module
//!
//! Input handling and command parsing for player interactions.

pub mod commands;

pub use commands::*;

use crate::{
    ConcreteAction, Direction, Entity, GameState, MoveAction, Position, ThatchError, ThatchResult,
    WaitAction,
};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;

/// Input handler for processing player commands.
///
/// Handles keyboard input and converts it to game actions that can be
/// processed by the game state.
pub struct InputHandler {
    /// Whether to enable Vi-style movement keys (hjkl)
    pub vi_keys_enabled: bool,
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

    /// Waits for and processes the next input event.
    ///
    /// Returns the corresponding game action, or None if no valid action
    /// was triggered by the input.
    pub fn wait_for_input(&self) -> ThatchResult<Option<PlayerInput>> {
        // Poll for events with a very short timeout for smooth rendering
        if event::poll(Duration::from_millis(1)).map_err(ThatchError::Io)? {
            match event::read().map_err(ThatchError::Io)? {
                Event::Key(key_event) => Ok(self.process_key_event(key_event)),
                Event::Resize(width, height) => Ok(Some(PlayerInput::Resize { width, height })),
                _ => Ok(None), // Ignore other event types for now
            }
        } else {
            Ok(None)
        }
    }

    /// Processes a keyboard event and returns the corresponding player input.
    fn process_key_event(&self, key_event: KeyEvent) -> Option<PlayerInput> {
        use crossterm::event::KeyEventKind;

        // Only process key press events, ignore key release events
        if key_event.kind != KeyEventKind::Press {
            return None;
        }

        match key_event.code {
            // Quit game
            KeyCode::Char('q') | KeyCode::Char('Q') => Some(PlayerInput::Quit),

            // Movement keys - Arrow keys
            KeyCode::Up => Some(PlayerInput::Move(Position::new(0, -1))),
            KeyCode::Down => Some(PlayerInput::Move(Position::new(0, 1))),
            KeyCode::Left => Some(PlayerInput::Move(Position::new(-1, 0))),
            KeyCode::Right => Some(PlayerInput::Move(Position::new(1, 0))),

            // Movement keys - Vi style (hjkl)
            KeyCode::Char('h') if self.vi_keys_enabled => {
                Some(PlayerInput::Move(Position::new(-1, 0)))
            }
            KeyCode::Char('j') if self.vi_keys_enabled => {
                Some(PlayerInput::Move(Position::new(0, 1)))
            }
            KeyCode::Char('k') if self.vi_keys_enabled => {
                Some(PlayerInput::Move(Position::new(0, -1)))
            }
            KeyCode::Char('l') if self.vi_keys_enabled => {
                Some(PlayerInput::Move(Position::new(1, 0)))
            }

            // Diagonal movement (Vi style)
            KeyCode::Char('y') if self.vi_keys_enabled => {
                Some(PlayerInput::Move(Position::new(-1, -1)))
            }
            KeyCode::Char('u') if self.vi_keys_enabled => {
                Some(PlayerInput::Move(Position::new(1, -1)))
            }
            KeyCode::Char('b') if self.vi_keys_enabled => {
                Some(PlayerInput::Move(Position::new(-1, 1)))
            }
            KeyCode::Char('n') if self.vi_keys_enabled => {
                Some(PlayerInput::Move(Position::new(1, 1)))
            }

            // Wait/rest
            KeyCode::Char('.') | KeyCode::Char(' ') => Some(PlayerInput::Wait),

            // Help
            KeyCode::Char('?') => Some(PlayerInput::Help),

            // Inventory
            KeyCode::Char('i') => Some(PlayerInput::ShowInventory),

            // Pick up item
            KeyCode::Char(',') | KeyCode::Char('g') => Some(PlayerInput::PickUp),

            // Escape (cancel current action)
            KeyCode::Esc => Some(PlayerInput::Cancel),

            // Enter (confirm action)
            KeyCode::Enter => Some(PlayerInput::Confirm),

            // Stairs
            KeyCode::Char('<') => Some(PlayerInput::UseStairs(crate::StairDirection::Up)),
            KeyCode::Char('>') => Some(PlayerInput::UseStairs(crate::StairDirection::Down)),

            // New game (when game ended)
            KeyCode::Char('n') | KeyCode::Char('N') => Some(PlayerInput::NewGame),

            _ => None, // Unrecognized key
        }
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
                    Ok(Some(ConcreteAction::UseStairs(crate::UseStairsAction::new(
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
    /// Terminal was resized
    Resize { width: u16, height: u16 },
    /// Use stairs in the specified direction
    UseStairs(crate::StairDirection),
    /// Start a new game (when game has ended)
    NewGame,
}
