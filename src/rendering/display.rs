//! # Display Management
//!
//! Screen management and terminal rendering functionality using crossterm.

use crate::{Entity, GameState, Level, Position, ThatchError, ThatchResult, TileType};
use crossterm::{
    cursor,
    event::{Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use std::io::{stdout, Write};

/// Terminal display manager for the game.
///
/// Handles all terminal rendering operations including map display,
/// UI elements, and screen management.
pub struct Display {
    /// Current terminal width
    pub width: u16,
    /// Current terminal height  
    pub height: u16,
    /// Map viewport offset x
    pub viewport_x: i32,
    /// Map viewport offset y
    pub viewport_y: i32,
    /// Map viewport width
    pub map_width: u16,
    /// Map viewport height
    pub map_height: u16,
    /// UI panel width (right side)
    pub ui_panel_width: u16,
    /// Message history
    pub messages: Vec<String>,
    /// Maximum number of messages to keep
    pub max_messages: usize,
    /// Previous frame buffer to avoid unnecessary redraws
    pub last_frame: Vec<Vec<CellData>>,
    /// Whether we need to redraw the entire screen
    pub needs_full_redraw: bool,
    /// Last player position for tracking movement
    pub last_player_pos: Option<Position>,
    /// Last message count for tracking new messages
    pub last_message_count: usize,
}

/// Data for a single terminal cell
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CellData {
    character: char,
    fg_color: Color,
    bg_color: Option<Color>,
}

impl Display {
    /// Creates a new display manager and initializes the terminal.
    ///
    /// # Examples
    ///
    /// ```
    /// use thatch::Display;
    ///
    /// let display = Display::new().unwrap();
    /// // Display is ready for rendering
    /// ```
    pub fn new() -> ThatchResult<Self> {
        // Get terminal size
        let (width, height) = terminal::size().map_err(|e| ThatchError::Io(e))?;

        // Calculate layout dimensions
        let ui_panel_width = 30; // Right panel for stats/inventory
        let map_width = width.saturating_sub(ui_panel_width + 1); // Leave space for border
        let map_height = height.saturating_sub(4); // Leave space for messages at bottom

        let mut display = Self {
            width,
            height,
            viewport_x: 0,
            viewport_y: 0,
            map_width,
            map_height,
            ui_panel_width,
            messages: Vec::new(),
            max_messages: 100,
            last_frame: vec![
                vec![
                    CellData {
                        character: ' ',
                        fg_color: Color::White,
                        bg_color: None
                    };
                    width as usize
                ];
                height as usize
            ],
            needs_full_redraw: true,
            last_player_pos: None,
            last_message_count: 0,
        };

        display.initialize_terminal()?;
        Ok(display)
    }

    /// Initializes the terminal for game display.
    pub fn initialize_terminal(&mut self) -> ThatchResult<()> {
        let mut stdout = stdout();

        // Enter alternate screen and enable raw mode
        execute!(stdout, EnterAlternateScreen).map_err(ThatchError::Io)?;
        terminal::enable_raw_mode().map_err(ThatchError::Io)?;

        // Clear the screen once, then rely on buffer-based rendering
        execute!(stdout, Clear(ClearType::All), cursor::Hide).map_err(ThatchError::Io)?;

        Ok(())
    }

    /// Restores terminal to normal state.
    pub fn cleanup(&mut self) -> ThatchResult<()> {
        let mut stdout = stdout();

        // Restore terminal state
        execute!(stdout, cursor::Show, ResetColor, LeaveAlternateScreen)
            .map_err(ThatchError::Io)?;
        terminal::disable_raw_mode().map_err(ThatchError::Io)?;

        Ok(())
    }

    /// Renders the complete game screen.
    ///
    /// This includes the map, UI panels, and message area.
    pub fn render_game(&mut self, game_state: &GameState) -> ThatchResult<()> {
        // Check if we need to update viewport
        let current_player_pos = game_state.get_player().map(|p| p.position());
        if current_player_pos != self.last_player_pos {
            if let Some(pos) = current_player_pos {
                self.center_viewport_on_position(pos);
            }
            self.last_player_pos = current_player_pos;
            self.needs_full_redraw = true;
        }

        // Check if messages changed
        if self.messages.len() != self.last_message_count {
            self.last_message_count = self.messages.len();
            self.needs_full_redraw = true; // For now, redraw all if messages change
        }

        // Create new frame buffer
        let mut new_frame = vec![
            vec![
                CellData {
                    character: ' ',
                    fg_color: Color::White,
                    bg_color: None
                };
                self.width as usize
            ];
            self.height as usize
        ];

        // Render to buffer
        self.render_map_to_buffer(&mut new_frame, game_state)?;
        self.render_ui_to_buffer(&mut new_frame, game_state)?;
        self.render_messages_to_buffer(&mut new_frame)?;
        self.render_border_to_buffer(&mut new_frame)?;

        // Only update changed cells
        self.update_changed_cells(&new_frame)?;

        // Store frame for next comparison
        self.last_frame = new_frame;
        self.needs_full_redraw = false;

        let mut stdout = stdout();
        stdout.flush().map_err(ThatchError::Io)?;
        Ok(())
    }

    /// Updates only the cells that have changed since last frame
    fn update_changed_cells(&self, new_frame: &[Vec<CellData>]) -> ThatchResult<()> {
        let mut stdout = stdout();

        for (y, row) in new_frame.iter().enumerate() {
            for (x, cell) in row.iter().enumerate() {
                // Skip if cell hasn't changed and we don't need full redraw
                if !self.needs_full_redraw
                    && y < self.last_frame.len()
                    && x < self.last_frame[y].len()
                    && &self.last_frame[y][x] == cell
                {
                    continue;
                }

                // Move cursor and write character
                execute!(
                    stdout,
                    cursor::MoveTo(x as u16, y as u16),
                    SetForegroundColor(cell.fg_color)
                )
                .map_err(ThatchError::Io)?;

                if let Some(bg_color) = cell.bg_color {
                    execute!(stdout, SetBackgroundColor(bg_color)).map_err(ThatchError::Io)?;
                }

                execute!(stdout, Print(cell.character)).map_err(ThatchError::Io)?;

                if cell.bg_color.is_some() {
                    execute!(stdout, SetBackgroundColor(Color::Reset)).map_err(ThatchError::Io)?;
                }
            }
        }

        Ok(())
    }

    /// Centers the viewport on the given position.
    pub fn center_viewport_on_position(&mut self, position: Position) {
        self.viewport_x = position.x - (self.map_width as i32 / 2);
        self.viewport_y = position.y - (self.map_height as i32 / 2);
    }

    /// Renders the game map to the frame buffer.
    fn render_map_to_buffer(
        &self,
        buffer: &mut [Vec<CellData>],
        game_state: &GameState,
    ) -> ThatchResult<()> {
        let level = game_state
            .world
            .current_level()
            .ok_or_else(|| ThatchError::InvalidState("No current level".to_string()))?;

        for screen_y in 0..self.map_height {
            for screen_x in 0..self.map_width {
                let world_x = self.viewport_x + screen_x as i32;
                let world_y = self.viewport_y + screen_y as i32;
                let world_pos = Position::new(world_x, world_y);

                let cell_data = if let Some(tile) = level.get_tile(world_pos) {
                    if tile.is_visible() {
                        self.get_tile_cell_data(game_state, world_pos, &tile.tile_type)
                    } else if tile.is_explored() {
                        // Render explored but not visible tiles in darker color
                        CellData {
                            character: self.get_tile_char(&tile.tile_type),
                            fg_color: Color::DarkGrey,
                            bg_color: None,
                        }
                    } else {
                        // Unexplored area
                        CellData {
                            character: ' ',
                            fg_color: Color::White,
                            bg_color: None,
                        }
                    }
                } else {
                    // Outside level bounds
                    CellData {
                        character: ' ',
                        fg_color: Color::White,
                        bg_color: None,
                    }
                };

                // Set in buffer
                if (screen_y as usize) < buffer.len()
                    && (screen_x as usize) < buffer[screen_y as usize].len()
                {
                    buffer[screen_y as usize][screen_x as usize] = cell_data;
                }
            }
        }

        Ok(())
    }

    /// Gets cell data for a tile at the given position, checking for entities.
    fn get_tile_cell_data(
        &self,
        game_state: &GameState,
        position: Position,
        tile_type: &TileType,
    ) -> CellData {
        // Check if there's an entity at this position
        if let Some(entity_id) = game_state.get_entity_at_position(position) {
            if let Some(entity) = game_state.entities.get(&entity_id) {
                let (character, color) = match entity {
                    crate::ConcreteEntity::Player(_) => ('@', Color::Yellow),
                };

                return CellData {
                    character,
                    fg_color: color,
                    bg_color: None,
                };
            }
        }

        // No entity, render the tile
        CellData {
            character: self.get_tile_char(tile_type),
            fg_color: self.get_tile_color(tile_type),
            bg_color: None,
        }
    }

    /// Gets the display character for a tile type.
    fn get_tile_char(&self, tile_type: &TileType) -> char {
        match tile_type {
            TileType::Wall => '#',
            TileType::Floor => '.',
            TileType::Door { is_open } => {
                if *is_open {
                    '\''
                } else {
                    '+'
                }
            }
            TileType::StairsUp => '<',
            TileType::StairsDown => '>',
            TileType::Water => '~',
            TileType::Special { .. } => '*',
        }
    }

    /// Gets the display color for a tile type.
    fn get_tile_color(&self, tile_type: &TileType) -> Color {
        match tile_type {
            TileType::Wall => Color::White,
            TileType::Floor => Color::Grey,
            TileType::Door { .. } => Color::Yellow,
            TileType::StairsUp => Color::Cyan,
            TileType::StairsDown => Color::Cyan,
            TileType::Water => Color::Blue,
            TileType::Special { .. } => Color::Magenta,
        }
    }

    /// Renders the UI panel to the buffer.
    fn render_ui_to_buffer(
        &self,
        buffer: &mut [Vec<CellData>],
        game_state: &GameState,
    ) -> ThatchResult<()> {
        let panel_x = self.map_width + 1;

        // Helper function to write text to buffer
        let mut write_text = |x: u16, y: u16, text: &str, color: Color| {
            for (i, ch) in text.chars().enumerate() {
                let pos_x = x as usize + i;
                let pos_y = y as usize;
                if pos_y < buffer.len() && pos_x < buffer[pos_y].len() {
                    buffer[pos_y][pos_x] = CellData {
                        character: ch,
                        fg_color: color,
                        bg_color: None,
                    };
                }
            }
        };

        // Render title
        write_text(panel_x, 0, "THATCH ROGUELIKE", Color::White);

        let mut line = 2;

        // Render player stats if available
        if let Some(player) = game_state.get_player() {
            write_text(
                panel_x,
                line,
                &format!("Player: {}", player.name),
                Color::Yellow,
            );
            line += 1;

            write_text(
                panel_x,
                line,
                &format!(
                    "Health: {}/{}",
                    player.stats.health, player.stats.max_health
                ),
                Color::White,
            );
            line += 1;

            write_text(
                panel_x,
                line,
                &format!("Mana: {}/{}", player.stats.mana, player.stats.max_mana),
                Color::White,
            );
            line += 1;

            write_text(
                panel_x,
                line,
                &format!("Level: {}", player.stats.level),
                Color::White,
            );
            line += 1;

            write_text(
                panel_x,
                line,
                &format!("XP: {}", player.stats.experience),
                Color::White,
            );
            line += 2;

            write_text(
                panel_x,
                line,
                &format!("Position: ({}, {})", player.position.x, player.position.y),
                Color::White,
            );
            line += 2;
        }

        // Render game info
        let time_info = game_state.get_game_time_info();
        write_text(panel_x, line, "Game Info:", Color::Cyan);
        line += 1;

        write_text(
            panel_x,
            line,
            &format!("Turn: {}", time_info.turn_number),
            Color::White,
        );
        line += 1;

        write_text(
            panel_x,
            line,
            &format!("Time: {}s", time_info.elapsed_time.as_secs()),
            Color::White,
        );
        line += 2;

        // Render controls
        write_text(panel_x, line, "Controls:", Color::Green);
        line += 1;

        let controls = ["hjkl/arrow keys: Move", "q: Quit", "?: Help"];

        for control in &controls {
            write_text(panel_x, line, control, Color::White);
            line += 1;
        }

        Ok(())
    }

    /// Renders the message area to the buffer.
    fn render_messages_to_buffer(&self, buffer: &mut [Vec<CellData>]) -> ThatchResult<()> {
        let message_area_y = self.height - 3;
        let message_count = 3; // Show last 3 messages

        // Render border
        for x in 0..self.width {
            let y = message_area_y - 1;
            if (y as usize) < buffer.len() && (x as usize) < buffer[y as usize].len() {
                buffer[y as usize][x as usize] = CellData {
                    character: '─',
                    fg_color: Color::White,
                    bg_color: None,
                };
            }
        }

        // Render messages
        let start_index = if self.messages.len() > message_count {
            self.messages.len() - message_count
        } else {
            0
        };

        for (i, message) in self.messages.iter().skip(start_index).enumerate() {
            let y = message_area_y + i as u16;
            for (x, ch) in message.chars().enumerate() {
                if (y as usize) < buffer.len() && x < buffer[y as usize].len() {
                    buffer[y as usize][x] = CellData {
                        character: ch,
                        fg_color: Color::White,
                        bg_color: None,
                    };
                }
            }
        }

        Ok(())
    }

    /// Renders the border between map and UI panel to the buffer.
    fn render_border_to_buffer(&self, buffer: &mut [Vec<CellData>]) -> ThatchResult<()> {
        let border_x = self.map_width;

        for y in 0..self.height {
            if (y as usize) < buffer.len() && (border_x as usize) < buffer[y as usize].len() {
                buffer[y as usize][border_x as usize] = CellData {
                    character: '│',
                    fg_color: Color::White,
                    bg_color: None,
                };
            }
        }

        Ok(())
    }

    /// Adds a message to the message history.
    pub fn add_message(&mut self, message: String) {
        self.messages.push(message);

        // Keep only the most recent messages
        if self.messages.len() > self.max_messages {
            self.messages.remove(0);
        }
    }

    /// Updates the display size if terminal was resized.
    pub fn update_size(&mut self) -> ThatchResult<()> {
        let (width, height) = terminal::size().map_err(ThatchError::Io)?;

        self.width = width;
        self.height = height;

        // Recalculate layout
        self.map_width = width.saturating_sub(self.ui_panel_width + 1);
        self.map_height = height.saturating_sub(4);

        // Resize frame buffer
        self.last_frame = vec![
            vec![
                CellData {
                    character: ' ',
                    fg_color: Color::White,
                    bg_color: None
                };
                width as usize
            ];
            height as usize
        ];
        self.needs_full_redraw = true;

        Ok(())
    }
}
