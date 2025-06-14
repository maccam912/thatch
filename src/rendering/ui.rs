//! # User Interface Elements
//!
//! UI components for health bars, inventory, messages, and other interface elements using macroquad.

use crate::game::{GameCompletionState, Position, StairDirection, TileType};
use crate::input::PlayerInput;
use crate::ThatchResult;
use macroquad::prelude::*;

/// UI component for rendering game screens.
pub struct UI;

impl Default for UI {
    fn default() -> Self {
        Self::new()
    }
}

impl UI {
    /// Creates a new UI instance.
    pub fn new() -> Self {
        Self
    }

    /// Renders the game over screen for early escape.
    pub async fn render_escape_screen(&self) -> ThatchResult<()> {
        clear_background(BLACK);

        let center_x = screen_width() / 2.0;
        let center_y = screen_height() / 2.0;

        // Title
        draw_text(
            "═══ ESCAPED ═══",
            center_x - 100.0,
            center_y - 120.0,
            32.0,
            YELLOW,
        );

        // Story text
        draw_text(
            "You emerge from the dungeon's entrance, gasping",
            center_x - 250.0,
            center_y - 70.0,
            20.0,
            WHITE,
        );
        draw_text(
            "for fresh air. Your life is saved, but you left",
            center_x - 250.0,
            center_y - 50.0,
            20.0,
            WHITE,
        );
        draw_text(
            "behind untold treasures in the depths below.",
            center_x - 250.0,
            center_y - 30.0,
            20.0,
            WHITE,
        );

        draw_text(
            "Sometimes living to fight another day is victory enough.",
            center_x - 250.0,
            center_y + 10.0,
            20.0,
            SKYBLUE,
        );

        // Controls
        draw_text(
            "Press 'N' for New Game",
            center_x - 120.0,
            center_y + 70.0,
            20.0,
            GREEN,
        );
        draw_text(
            "Press 'ESC' to Quit",
            center_x - 120.0,
            center_y + 90.0,
            20.0,
            GREEN,
        );

        Ok(())
    }

    /// Renders the victory screen for completing the dungeon.
    pub async fn render_victory_screen(&self) -> ThatchResult<()> {
        clear_background(BLACK);

        let center_x = screen_width() / 2.0;
        let center_y = screen_height() / 2.0;

        // Title
        draw_text(
            "♦═══ VICTORY! ═══♦",
            center_x - 120.0,
            center_y - 120.0,
            32.0,
            MAGENTA,
        );

        // Story text
        draw_text(
            "You have conquered the deepest depths of the ancient",
            center_x - 300.0,
            center_y - 70.0,
            20.0,
            WHITE,
        );
        draw_text(
            "dungeon! The treasures of 26 levels are yours, and",
            center_x - 300.0,
            center_y - 50.0,
            20.0,
            WHITE,
        );
        draw_text(
            "your name will be sung by bards for generations.",
            center_x - 300.0,
            center_y - 30.0,
            20.0,
            WHITE,
        );

        draw_text(
            "You are a true master of the depths!",
            center_x - 200.0,
            center_y + 10.0,
            20.0,
            YELLOW,
        );

        // Controls
        draw_text(
            "Press 'N' for New Game",
            center_x - 120.0,
            center_y + 70.0,
            20.0,
            GREEN,
        );
        draw_text(
            "Press 'ESC' to Quit",
            center_x - 120.0,
            center_y + 90.0,
            20.0,
            GREEN,
        );

        Ok(())
    }

    /// Renders the death screen when the player dies.
    pub async fn render_death_screen(&self) -> ThatchResult<()> {
        clear_background(BLACK);

        let center_x = screen_width() / 2.0;
        let center_y = screen_height() / 2.0;

        // Title
        draw_text(
            "═══ YOU DIED ═══",
            center_x - 120.0,
            center_y - 120.0,
            32.0,
            RED,
        );

        // Story text
        draw_text(
            "Your adventure ends here in the depths of the dungeon.",
            center_x - 280.0,
            center_y - 70.0,
            20.0,
            WHITE,
        );
        draw_text(
            "Death is not the end, but a new beginning. Learn from",
            center_x - 280.0,
            center_y - 50.0,
            20.0,
            WHITE,
        );
        draw_text(
            "your mistakes and return stronger than before.",
            center_x - 280.0,
            center_y - 30.0,
            20.0,
            WHITE,
        );

        draw_text(
            "The dungeon awaits your return...",
            center_x - 160.0,
            center_y + 10.0,
            20.0,
            DARKGRAY,
        );

        // Controls
        draw_text(
            "Press 'N' for New Game",
            center_x - 120.0,
            center_y + 70.0,
            20.0,
            GREEN,
        );
        draw_text(
            "Press 'ESC' to Quit",
            center_x - 120.0,
            center_y + 90.0,
            20.0,
            GREEN,
        );

        Ok(())
    }

    /// Renders tooltips for special tiles.
    pub fn render_tile_tooltip(&self, tile_type: &TileType, x: f32, y: f32) -> ThatchResult<()> {
        let tooltip_text = match tile_type {
            TileType::StairsUp => {
                "Stairs Up - Press '1' to ascend (Warning: Exiting at level 1 ends the game!)"
            }
            TileType::StairsDown => "Stairs Down - Press '2' to descend to the next level",
            TileType::Door { is_open } => {
                if *is_open {
                    "Open Door - Press 'C' to close"
                } else {
                    "Closed Door - Press 'O' to open"
                }
            }
            TileType::Special { description } => description,
            _ => return Ok(()), // No tooltip for regular tiles
        };

        // Render tooltip box with background
        let text_width = tooltip_text.len() as f32 * 8.0;
        draw_rectangle(
            x,
            y - 20.0,
            text_width + 10.0,
            25.0,
            Color::new(0.0, 0.0, 0.5, 0.8),
        );
        draw_text(tooltip_text, x + 5.0, y - 5.0, 16.0, WHITE);

        Ok(())
    }

    /// Renders the game ending screen based on completion state.
    pub async fn render_ending_screen(
        &self,
        completion_state: &GameCompletionState,
    ) -> ThatchResult<()> {
        match completion_state {
            GameCompletionState::EscapedEarly => self.render_escape_screen().await,
            GameCompletionState::CompletedDungeon => self.render_victory_screen().await,
            GameCompletionState::PlayerDied => self.render_death_screen().await,
            GameCompletionState::Playing => {
                // Should not render ending screen if still playing
                Ok(())
            }
        }
    }

    /// Renders touch-friendly control buttons and handles touch input.
    ///
    /// Returns the player input if a button was pressed, None otherwise.
    pub fn render_touch_controls(&self) -> Option<PlayerInput> {
        let screen_w = screen_width();
        let screen_h = screen_height();

        // Button dimensions - increased for better touch targets
        let button_size = 70.0;
        let button_margin = 12.0;

        // Movement pad (left side)
        let pad_x = button_margin;
        let pad_y = screen_h - (button_size * 3.0 + button_margin * 4.0);

        // Check movement buttons
        if let Some(input) = self.render_movement_pad(pad_x, pad_y, button_size, button_margin) {
            return Some(input);
        }

        // Action buttons (right side)
        let action_x = screen_w - (button_size * 2.0 + button_margin * 3.0);
        let action_y = screen_h - (button_size * 3.0 + button_margin * 4.0);

        if let Some(input) =
            self.render_action_buttons(action_x, action_y, button_size, button_margin)
        {
            return Some(input);
        }

        None
    }

    /// Renders the movement directional pad.
    fn render_movement_pad(&self, x: f32, y: f32, size: f32, margin: f32) -> Option<PlayerInput> {
        let mut input = None;

        // Bright blue for better visibility on Android
        let move_color = Color::new(0.0, 0.4, 1.0, 1.0);
        let wait_color = Color::new(0.6, 0.6, 0.6, 1.0);

        // Up button
        if self.render_button("↑", x + size + margin, y, size, size, move_color) {
            input = Some(PlayerInput::Move(Position::new(0, -1)));
        }

        // Left button
        if self.render_button("←", x, y + size + margin, size, size, move_color) {
            input = Some(PlayerInput::Move(Position::new(-1, 0)));
        }

        // Center (wait) button
        if self.render_button(
            "WAIT",
            x + size + margin,
            y + size + margin,
            size,
            size,
            wait_color,
        ) {
            input = Some(PlayerInput::Wait);
        }

        // Right button
        if self.render_button(
            "→",
            x + (size + margin) * 2.0,
            y + size + margin,
            size,
            size,
            move_color,
        ) {
            input = Some(PlayerInput::Move(Position::new(1, 0)));
        }

        // Down button
        if self.render_button(
            "↓",
            x + size + margin,
            y + (size + margin) * 2.0,
            size,
            size,
            move_color,
        ) {
            input = Some(PlayerInput::Move(Position::new(0, 1)));
        }

        input
    }

    /// Renders action buttons for stairs and autoexplore.
    fn render_action_buttons(&self, x: f32, y: f32, size: f32, margin: f32) -> Option<PlayerInput> {
        let mut input = None;

        // Up stairs button - bright green for better visibility
        if self.render_button("UP", x, y, size, size, Color::new(0.0, 0.8, 0.0, 1.0)) {
            input = Some(PlayerInput::UseStairs(StairDirection::Up));
        }

        // Down stairs button - bright green for better visibility
        if self.render_button(
            "DN",
            x + size + margin,
            y,
            size,
            size,
            Color::new(0.0, 0.8, 0.0, 1.0),
        ) {
            input = Some(PlayerInput::UseStairs(StairDirection::Down));
        }

        // Autoexplore button - bright purple for better visibility
        if self.render_button(
            "AUTO",
            x,
            y + size + margin,
            size,
            size,
            Color::new(0.8, 0.0, 0.8, 1.0),
        ) {
            input = Some(PlayerInput::ToggleAutoexplore);
        }

        // Help button - bright orange for better visibility
        if self.render_button(
            "HELP",
            x + size + margin,
            y + size + margin,
            size,
            size,
            Color::new(1.0, 0.6, 0.0, 1.0),
        ) {
            input = Some(PlayerInput::Help);
        }

        input
    }

    /// Renders a single button and returns true if it was pressed.
    fn render_button(
        &self,
        text: &str,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: Color,
    ) -> bool {
        let mouse_pos = mouse_position();
        let is_hovered = mouse_pos.0 >= x
            && mouse_pos.0 <= x + width
            && mouse_pos.1 >= y
            && mouse_pos.1 <= y + height;

        let button_color = if is_hovered {
            Color::new(
                (color.r * 1.5).min(1.0),
                (color.g * 1.5).min(1.0),
                (color.b * 1.5).min(1.0),
                color.a,
            )
        } else {
            color
        };

        // Draw button background with better contrast
        draw_rectangle(x, y, width, height, button_color);
        draw_rectangle_lines(x, y, width, height, 3.0, WHITE);

        // Add inner shadow for better visibility
        draw_rectangle_lines(x + 1.0, y + 1.0, width - 2.0, height - 2.0, 1.0, LIGHTGRAY);

        // Draw button text with better contrast
        let text_size = 28.0; // Larger text for better visibility
        let text_width = text.len() as f32 * text_size * 0.6;
        let text_x = x + (width - text_width) / 2.0;
        let text_y = y + height / 2.0 + text_size / 2.0;

        // Draw text with outline for better visibility on Android
        draw_text(text, text_x - 1.0, text_y - 1.0, text_size, BLACK);
        draw_text(text, text_x + 1.0, text_y - 1.0, text_size, BLACK);
        draw_text(text, text_x - 1.0, text_y + 1.0, text_size, BLACK);
        draw_text(text, text_x + 1.0, text_y + 1.0, text_size, BLACK);
        draw_text(text, text_x, text_y, text_size, WHITE);

        // Check if button was pressed
        is_hovered && is_mouse_button_pressed(MouseButton::Left)
    }
}
