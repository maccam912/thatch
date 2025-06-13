//! # User Interface Elements
//!
//! UI components for health bars, inventory, messages, and other interface elements.

use crate::{GameCompletionState, ThatchResult};
use crossterm::{
    cursor,
    style::{Color, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal, ExecutableCommand,
};
use std::io::Write;

/// UI component for rendering game screens.
pub struct UI;

impl UI {
    /// Creates a new UI instance.
    pub fn new() -> Self {
        Self
    }

    /// Renders the game over screen for early escape.
    pub fn render_escape_screen<W: Write>(&self, writer: &mut W) -> ThatchResult<()> {
        self.clear_screen(writer)?;
        
        let (width, height) = terminal::size()?;
        let center_x = width / 2;
        let center_y = height / 2;

        // Title
        writer.execute(cursor::MoveTo(center_x - 10, center_y - 8))?;
        writer.execute(SetForegroundColor(Color::Yellow))?;
        writeln!(writer, "═══ ESCAPED ═══")?;

        // Story text
        writer.execute(cursor::MoveTo(center_x - 25, center_y - 5))?;
        writer.execute(SetForegroundColor(Color::White))?;
        writeln!(writer, "You emerge from the dungeon's entrance, gasping")?;
        writer.execute(cursor::MoveTo(center_x - 25, center_y - 4))?;
        writeln!(writer, "for fresh air. Your life is saved, but you left")?;
        writer.execute(cursor::MoveTo(center_x - 25, center_y - 3))?;
        writeln!(writer, "behind untold treasures in the depths below.")?;

        writer.execute(cursor::MoveTo(center_x - 25, center_y - 1))?;
        writer.execute(SetForegroundColor(Color::Cyan))?;
        writeln!(writer, "Sometimes living to fight another day is victory enough.")?;

        // Controls
        writer.execute(cursor::MoveTo(center_x - 15, center_y + 3))?;
        writer.execute(SetForegroundColor(Color::Green))?;
        writeln!(writer, "Press 'n' for New Game")?;
        writer.execute(cursor::MoveTo(center_x - 15, center_y + 4))?;
        writeln!(writer, "Press 'q' to Quit")?;

        writer.execute(ResetColor)?;
        writer.flush()?;
        Ok(())
    }

    /// Renders the victory screen for completing the dungeon.
    pub fn render_victory_screen<W: Write>(&self, writer: &mut W) -> ThatchResult<()> {
        self.clear_screen(writer)?;
        
        let (width, height) = terminal::size()?;
        let center_x = width / 2;
        let center_y = height / 2;

        // Title
        writer.execute(cursor::MoveTo(center_x - 12, center_y - 8))?;
        writer.execute(SetForegroundColor(Color::Magenta))?;
        writeln!(writer, "♦═══ VICTORY! ═══♦")?;

        // Story text
        writer.execute(cursor::MoveTo(center_x - 30, center_y - 5))?;
        writer.execute(SetForegroundColor(Color::White))?;
        writeln!(writer, "You have conquered the deepest depths of the ancient")?;
        writer.execute(cursor::MoveTo(center_x - 30, center_y - 4))?;
        writeln!(writer, "dungeon! The treasures of 26 levels are yours, and")?;
        writer.execute(cursor::MoveTo(center_x - 30, center_y - 3))?;
        writeln!(writer, "your name will be sung by bards for generations.")?;

        writer.execute(cursor::MoveTo(center_x - 20, center_y - 1))?;
        writer.execute(SetForegroundColor(Color::Yellow))?;
        writeln!(writer, "You are a true master of the depths!")?;

        // Controls
        writer.execute(cursor::MoveTo(center_x - 15, center_y + 3))?;
        writer.execute(SetForegroundColor(Color::Green))?;
        writeln!(writer, "Press 'n' for New Game")?;
        writer.execute(cursor::MoveTo(center_x - 15, center_y + 4))?;
        writeln!(writer, "Press 'q' to Quit")?;

        writer.execute(ResetColor)?;
        writer.flush()?;
        Ok(())
    }

    /// Renders tooltips for special tiles.
    pub fn render_tile_tooltip<W: Write>(
        &self,
        writer: &mut W,
        tile_type: &crate::TileType,
        x: u16,
        y: u16,
    ) -> ThatchResult<()> {
        let tooltip_text = match tile_type {
            crate::TileType::StairsUp => {
                "Stairs Up - Press '<' to ascend (Warning: Exiting at level 1 ends the game!)"
            }
            crate::TileType::StairsDown => {
                "Stairs Down - Press '>' to descend to the next level"
            }
            crate::TileType::Door { is_open } => {
                if *is_open {
                    "Open Door - Press 'c' to close"
                } else {
                    "Closed Door - Press 'o' to open"
                }
            }
            crate::TileType::Special { description } => description,
            _ => return Ok(()), // No tooltip for regular tiles
        };

        // Render tooltip box
        writer.execute(cursor::MoveTo(x, y))?;
        writer.execute(SetBackgroundColor(Color::DarkBlue))?;
        writer.execute(SetForegroundColor(Color::White))?;
        write!(writer, " {} ", tooltip_text)?;
        writer.execute(ResetColor)?;
        
        Ok(())
    }

    /// Renders the game ending screen based on completion state.
    pub fn render_ending_screen<W: Write>(
        &self,
        writer: &mut W,
        completion_state: &GameCompletionState,
    ) -> ThatchResult<()> {
        match completion_state {
            GameCompletionState::EscapedEarly => self.render_escape_screen(writer),
            GameCompletionState::CompletedDungeon => self.render_victory_screen(writer),
            GameCompletionState::PlayerDied => {
                // TODO: Implement death screen
                self.render_escape_screen(writer) // Placeholder
            }
            GameCompletionState::Playing => {
                // Should not render ending screen if still playing
                Ok(())
            }
        }
    }

    /// Clears the screen.
    fn clear_screen<W: Write>(&self, writer: &mut W) -> ThatchResult<()> {
        writer.execute(terminal::Clear(terminal::ClearType::All))?;
        writer.execute(cursor::MoveTo(0, 0))?;
        Ok(())
    }
}
