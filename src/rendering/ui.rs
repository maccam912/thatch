//! # User Interface Elements
//!
//! UI components for health bars, inventory, messages, and other interface elements using macroquad.

use crate::{GameCompletionState, ThatchResult};
use macroquad::prelude::*;

/// UI component for rendering game screens.
pub struct UI;

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
        draw_text("═══ ESCAPED ═══", center_x - 100.0, center_y - 120.0, 32.0, YELLOW);

        // Story text
        draw_text("You emerge from the dungeon's entrance, gasping", center_x - 250.0, center_y - 70.0, 20.0, WHITE);
        draw_text("for fresh air. Your life is saved, but you left", center_x - 250.0, center_y - 50.0, 20.0, WHITE);
        draw_text("behind untold treasures in the depths below.", center_x - 250.0, center_y - 30.0, 20.0, WHITE);

        draw_text("Sometimes living to fight another day is victory enough.", center_x - 250.0, center_y + 10.0, 20.0, SKYBLUE);

        // Controls
        draw_text("Press 'N' for New Game", center_x - 120.0, center_y + 70.0, 20.0, GREEN);
        draw_text("Press 'ESC' to Quit", center_x - 120.0, center_y + 90.0, 20.0, GREEN);

        Ok(())
    }

    /// Renders the victory screen for completing the dungeon.
    pub async fn render_victory_screen(&self) -> ThatchResult<()> {
        clear_background(BLACK);
        
        let center_x = screen_width() / 2.0;
        let center_y = screen_height() / 2.0;

        // Title
        draw_text("♦═══ VICTORY! ═══♦", center_x - 120.0, center_y - 120.0, 32.0, MAGENTA);

        // Story text
        draw_text("You have conquered the deepest depths of the ancient", center_x - 300.0, center_y - 70.0, 20.0, WHITE);
        draw_text("dungeon! The treasures of 26 levels are yours, and", center_x - 300.0, center_y - 50.0, 20.0, WHITE);
        draw_text("your name will be sung by bards for generations.", center_x - 300.0, center_y - 30.0, 20.0, WHITE);

        draw_text("You are a true master of the depths!", center_x - 200.0, center_y + 10.0, 20.0, YELLOW);

        // Controls
        draw_text("Press 'N' for New Game", center_x - 120.0, center_y + 70.0, 20.0, GREEN);
        draw_text("Press 'ESC' to Quit", center_x - 120.0, center_y + 90.0, 20.0, GREEN);

        Ok(())
    }

    /// Renders tooltips for special tiles.
    pub fn render_tile_tooltip(
        &self,
        tile_type: &crate::TileType,
        x: f32,
        y: f32,
    ) -> ThatchResult<()> {
        let tooltip_text = match tile_type {
            crate::TileType::StairsUp => {
                "Stairs Up - Press '1' to ascend (Warning: Exiting at level 1 ends the game!)"
            }
            crate::TileType::StairsDown => {
                "Stairs Down - Press '2' to descend to the next level"
            }
            crate::TileType::Door { is_open } => {
                if *is_open {
                    "Open Door - Press 'C' to close"
                } else {
                    "Closed Door - Press 'O' to open"
                }
            }
            crate::TileType::Special { description } => description,
            _ => return Ok(()), // No tooltip for regular tiles
        };

        // Render tooltip box with background
        let text_width = tooltip_text.len() as f32 * 8.0;
        draw_rectangle(x, y - 20.0, text_width + 10.0, 25.0, Color::new(0.0, 0.0, 0.5, 0.8));
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
            GameCompletionState::PlayerDied => {
                // TODO: Implement death screen
                self.render_escape_screen().await // Placeholder
            }
            GameCompletionState::Playing => {
                // Should not render ending screen if still playing
                Ok(())
            }
        }
    }
}