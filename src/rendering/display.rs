//! # Display Management
//!
//! Screen management and 2D graphics rendering functionality using macroquad.

use crate::game::{ConcreteEntity, Entity, GameState, Position, TileType};
use crate::input::PlayerInput;
use crate::rendering::UI;
use crate::{ThatchError, ThatchResult};
use macroquad::prelude::*;
use std::collections::HashMap;

/// Macroquad display manager for the game.
///
/// Handles all 2D graphics rendering operations including map display,
/// UI elements, and screen management.
pub struct MacroquadDisplay {
    /// Screen width in pixels
    pub screen_width: f32,
    /// Screen height in pixels
    pub screen_height: f32,
    /// Tile size in pixels
    pub tile_size: f32,
    /// Map viewport offset x in tiles
    pub viewport_x: i32,
    /// Map viewport offset y in tiles
    pub viewport_y: i32,
    /// Map viewport width in tiles
    pub map_width: i32,
    /// Map viewport height in tiles
    pub map_height: i32,
    /// UI panel width in pixels
    pub ui_panel_width: f32,
    /// Message history
    pub messages: Vec<String>,
    /// Maximum number of messages to keep
    pub max_messages: usize,
    /// Last player position for tracking movement
    pub last_player_pos: Option<Position>,
    /// Tile textures
    pub tile_textures: HashMap<char, Texture2D>,
    /// Font for text rendering
    pub font: Option<Font>,
    /// UI component for touch controls
    pub ui: UI,
}

impl MacroquadDisplay {
    /// Creates a new display manager and initializes macroquad rendering.
    ///
    /// # Examples
    ///
    /// ```
    /// use thatch::MacroquadDisplay;
    ///
    /// let display = MacroquadDisplay::new().await.unwrap();
    /// // Display is ready for rendering
    /// ```
    pub async fn new() -> ThatchResult<Self> {
        let mut display = Self {
            screen_width: 0.0,
            screen_height: 0.0,
            tile_size: 0.0,
            viewport_x: 0,
            viewport_y: 0,
            map_width: 0,
            map_height: 0,
            ui_panel_width: 0.0,
            messages: Vec::new(),
            max_messages: 100,
            last_player_pos: None,
            tile_textures: HashMap::new(),
            font: None,
            ui: UI::new(),
        };

        display.update_layout_dimensions();
        display.initialize_graphics().await?;
        Ok(display)
    }

    /// Draws text that wraps within a specified width.
    fn draw_wrapped_text(&self, text: &str, x: f32, y: f32, font_size: f32, color: Color, _max_width: f32) {
        // For now, just draw the text normally (word wrapping is complex)
        // In a real implementation, you'd break text into lines
        draw_text(text, x, y, font_size, color);
    }

    /// Updates layout dimensions based on current screen size for responsive design.
    pub fn update_layout_dimensions(&mut self) {
        let current_width = screen_width();
        let current_height = screen_height();

        // Only update if screen size changed or first time
        if (current_width - self.screen_width).abs() > 1.0 || (current_height - self.screen_height).abs() > 1.0 {
            self.screen_width = current_width;
            self.screen_height = current_height;

            // Responsive calculations
            self.calculate_responsive_layout();
        }
    }

    /// Calculates responsive layout dimensions based on screen size.
    fn calculate_responsive_layout(&mut self) {
        // Responsive tile size based on screen resolution
        let base_tile_size = 24.0;
        let scale_factor = (self.screen_width / 1024.0).max(0.5).min(2.0); // Scale between 0.5x and 2x
        self.tile_size = base_tile_size * scale_factor;

        // Responsive UI panel width (15-25% of screen width)
        let panel_ratio = if self.screen_width < 800.0 { 0.15 } else if self.screen_width > 1600.0 { 0.20 } else { 0.18 };
        self.ui_panel_width = (self.screen_width * panel_ratio).max(250.0).min(400.0);

        // Message area height (8-12% of screen height)
        let message_ratio = if self.screen_height < 600.0 { 0.08 } else { 0.10 };
        let message_area_height = (self.screen_height * message_ratio).max(60.0).min(120.0);

        // Calculate map dimensions
        let available_map_width = self.screen_width - self.ui_panel_width;
        let available_map_height = self.screen_height - message_area_height;

        self.map_width = (available_map_width / self.tile_size) as i32;
        self.map_height = (available_map_height / self.tile_size) as i32;

        // Ensure minimum map size
        self.map_width = self.map_width.max(20);
        self.map_height = self.map_height.max(15);
    }

    /// Initializes graphics resources.
    async fn initialize_graphics(&mut self) -> ThatchResult<()> {
        // Create simple tile textures using rectangles
        self.create_tile_textures().await;

        Ok(())
    }

    /// Creates tile textures for different tile types.
    async fn create_tile_textures(&mut self) {
        // For now, we'll just use colored rectangles for tiles
        // In a real implementation, you'd load actual texture files

        // Create a simple 1x1 white texture that we can tint
        let white_texture = Texture2D::from_rgba8(1, 1, &[255, 255, 255, 255]);

        // Map characters to the base texture (we'll use colors to differentiate)
        // Note: In macroquad, textures are reference-counted, so cloning is cheap
        self.tile_textures.insert('#', white_texture.clone()); // Wall
        self.tile_textures.insert('.', white_texture.clone()); // Floor
        self.tile_textures.insert('@', white_texture.clone()); // Player
        self.tile_textures.insert('+', white_texture.clone()); // Closed door
        self.tile_textures.insert('\'', white_texture.clone()); // Open door
        self.tile_textures.insert('<', white_texture.clone()); // Stairs up
        self.tile_textures.insert('>', white_texture.clone()); // Stairs down
        self.tile_textures.insert('~', white_texture.clone()); // Water
        self.tile_textures.insert('*', white_texture); // Special
    }

    /// Renders the complete game screen.
    ///
    /// This includes the map, UI panels, message area, and touch controls.
    pub async fn render_game(&mut self, game_state: &GameState) -> ThatchResult<()> {
        // Update layout dimensions for responsive design
        self.update_layout_dimensions();

        // Check if we need to update viewport
        let current_player_pos = game_state.get_player().map(|p| p.position());
        if current_player_pos != self.last_player_pos {
            if let Some(pos) = current_player_pos {
                self.center_viewport_on_position(pos);
            }
            self.last_player_pos = current_player_pos;
        }

        // Clear screen
        clear_background(BLACK);

        // Render components
        self.render_map(game_state)?;
        self.render_ui(game_state)?;
        self.render_messages()?;

        // Always render touch controls for all platforms
        self.ui.render_touch_controls();

        Ok(())
    }

    /// Centers the viewport on the given position.
    pub fn center_viewport_on_position(&mut self, position: Position) {
        self.viewport_x = position.x - (self.map_width / 2);
        self.viewport_y = position.y - (self.map_height / 2);
    }

    /// Renders the game map using macroquad graphics.
    fn render_map(&self, game_state: &GameState) -> ThatchResult<()> {
        let level = game_state
            .world
            .current_level()
            .ok_or_else(|| ThatchError::InvalidState("No current level".to_string()))?;

        for screen_y in 0..self.map_height {
            for screen_x in 0..self.map_width {
                let world_x = self.viewport_x + screen_x;
                let world_y = self.viewport_y + screen_y;
                let world_pos = Position::new(world_x, world_y);

                let screen_pixel_x = screen_x as f32 * self.tile_size;
                let screen_pixel_y = screen_y as f32 * self.tile_size;

                if let Some(tile) = level.get_tile(world_pos) {
                    if tile.is_visible() {
                        self.render_tile_at_position(
                            game_state,
                            world_pos,
                            &tile.tile_type,
                            screen_pixel_x,
                            screen_pixel_y,
                            false,
                        );
                    } else if tile.is_explored() {
                        // Render explored but not visible tiles in darker color
                        self.render_tile_at_position(
                            game_state,
                            world_pos,
                            &tile.tile_type,
                            screen_pixel_x,
                            screen_pixel_y,
                            true,
                        );
                    }
                    // Don't render unexplored tiles (leave them black)
                }
            }
        }

        Ok(())
    }

    /// Renders a tile at the given screen position.
    fn render_tile_at_position(
        &self,
        game_state: &GameState,
        world_pos: Position,
        tile_type: &TileType,
        screen_x: f32,
        screen_y: f32,
        is_explored_only: bool,
    ) {
        // Check if there's an entity at this position
        if let Some(entity_id) = game_state.get_entity_at_position(world_pos) {
            if let Some(entity) = game_state.entities.get(&entity_id) {
                let (character, base_color) = match entity {
                    ConcreteEntity::Player(_) => ('@', YELLOW),
                };

                let color = if is_explored_only {
                    Color::new(
                        base_color.r * 0.4,
                        base_color.g * 0.4,
                        base_color.b * 0.4,
                        base_color.a,
                    )
                } else {
                    base_color
                };

                if let Some(texture) = self.tile_textures.get(&character) {
                    draw_texture_ex(
                        *texture,
                        screen_x,
                        screen_y,
                        color,
                        DrawTextureParams {
                            dest_size: Some(vec2(self.tile_size, self.tile_size)),
                            ..Default::default()
                        },
                    );
                }
                return;
            }
        }

        // No entity, render the tile
        let (character, base_color) = self.get_tile_display_data(tile_type);
        let color = if is_explored_only {
            Color::new(
                base_color.r * 0.4,
                base_color.g * 0.4,
                base_color.b * 0.4,
                base_color.a,
            )
        } else {
            base_color
        };

        if let Some(texture) = self.tile_textures.get(&character) {
            draw_texture_ex(
                *texture,
                screen_x,
                screen_y,
                color,
                DrawTextureParams {
                    dest_size: Some(vec2(self.tile_size, self.tile_size)),
                    ..Default::default()
                },
            );
        }
    }

    /// Gets the display character and color for a tile type.
    fn get_tile_display_data(&self, tile_type: &TileType) -> (char, Color) {
        match tile_type {
            TileType::Wall => ('#', WHITE),
            TileType::Floor => ('.', GRAY),
            TileType::Door { is_open } => {
                if *is_open {
                    ('\'', YELLOW)
                } else {
                    ('+', YELLOW)
                }
            }
            TileType::StairsUp => ('<', LIGHTGRAY),
            TileType::StairsDown => ('>', ORANGE),
            TileType::Water => ('~', BLUE),
            TileType::Special { .. } => ('*', MAGENTA),
        }
    }

    /// Renders the UI panel.
    fn render_ui(&self, game_state: &GameState) -> ThatchResult<()> {
        let panel_x = self.map_width as f32 * self.tile_size + 10.0;
        let panel_width = self.ui_panel_width - 20.0; // Leave margins
        let mut line_y = 20.0;
        
        // Responsive font sizes and spacing
        let scale_factor = (self.screen_width / 1024.0).max(0.7).min(1.3);
        let title_font_size = 24.0 * scale_factor;
        let normal_font_size = 16.0 * scale_factor;
        let line_height = 18.0 * scale_factor;

        // Render panel background
        draw_rectangle(
            panel_x - 5.0,
            0.0,
            self.ui_panel_width,
            self.screen_height,
            Color::new(0.1, 0.1, 0.1, 0.8),
        );

        // Render title
        draw_text("THATCH ROGUELIKE", panel_x, line_y, title_font_size, WHITE);
        line_y += line_height * 2.0;

        // Render player stats if available
        if let Some(player) = game_state.get_player() {
            self.draw_wrapped_text(
                &format!("Player: {}", player.name),
                panel_x,
                line_y,
                normal_font_size,
                YELLOW,
                panel_width,
            );
            line_y += line_height;

            self.draw_wrapped_text(
                &format!(
                    "Health: {}/{}",
                    player.stats.health, player.stats.max_health
                ),
                panel_x,
                line_y,
                normal_font_size,
                WHITE,
                panel_width,
            );
            line_y += line_height;

            self.draw_wrapped_text(
                &format!("Mana: {}/{}", player.stats.mana, player.stats.max_mana),
                panel_x,
                line_y,
                normal_font_size,
                WHITE,
                panel_width,
            );
            line_y += line_height;

            self.draw_wrapped_text(
                &format!("Dungeon Level: {}", game_state.world.current_level_id + 1),
                panel_x,
                line_y,
                normal_font_size,
                WHITE,
                panel_width,
            );
            line_y += line_height;

            self.draw_wrapped_text(
                &format!("Character Level: {}", player.stats.level),
                panel_x,
                line_y,
                normal_font_size,
                WHITE,
                panel_width,
            );
            line_y += line_height;

            self.draw_wrapped_text(
                &format!("XP: {}", player.stats.experience),
                panel_x,
                line_y,
                normal_font_size,
                WHITE,
                panel_width,
            );
            line_y += line_height * 2.0;

            self.draw_wrapped_text(
                &format!("Position: ({}, {})", player.position.x, player.position.y),
                panel_x,
                line_y,
                normal_font_size,
                WHITE,
                panel_width,
            );
            line_y += line_height;

            // Show tile information
            if let Some(level) = game_state.world.current_level() {
                if let Some(tile) = level.get_tile(player.position()) {
                    let tile_name = match &tile.tile_type {
                        TileType::Floor => "Floor",
                        TileType::Wall => "Wall",
                        TileType::Door { is_open } => {
                            if *is_open {
                                "Open Door"
                            } else {
                                "Closed Door"
                            }
                        }
                        TileType::StairsUp => "Stairs Up",
                        TileType::StairsDown => "Stairs Down",
                        TileType::Water => "Water",
                        TileType::Special { .. } => "Special",
                    };

                    let tile_color = match &tile.tile_type {
                        TileType::StairsUp => LIGHTGRAY,
                        TileType::StairsDown => ORANGE,
                        _ => WHITE,
                    };

                    self.draw_wrapped_text(
                        &format!("Standing on: {}", tile_name),
                        panel_x,
                        line_y,
                        normal_font_size,
                        tile_color,
                        panel_width,
                    );
                }
            }
            line_y += line_height * 2.0;
        }

        // Render game info
        let time_info = game_state.get_game_time_info();
        self.draw_wrapped_text("Game Info:", panel_x, line_y, normal_font_size, SKYBLUE, panel_width);
        line_y += line_height;

        self.draw_wrapped_text(
            &format!("Turn: {}", time_info.turn_number),
            panel_x,
            line_y,
            normal_font_size,
            WHITE,
            panel_width,
        );
        line_y += line_height;

        self.draw_wrapped_text(
            &format!("Time: {}s", time_info.elapsed_time.as_secs()),
            panel_x,
            line_y,
            normal_font_size,
            WHITE,
            panel_width,
        );
        line_y += line_height * 2.0;

        // Render controls
        self.draw_wrapped_text("Controls:", panel_x, line_y, normal_font_size, GREEN, panel_width);
        line_y += line_height;

        // Always available controls
        let basic_controls = [
            "WASD/Arrow keys: Move",
            "SPACE: Wait",
            "ESC: Quit",
            "F1: Help",
        ];

        for control in &basic_controls {
            self.draw_wrapped_text(control, panel_x, line_y, normal_font_size, WHITE, panel_width);
            line_y += line_height;
        }

        // Conditional stair controls based on player position
        if let Some(player) = game_state.get_player() {
            if let Some(level) = game_state.world.current_level() {
                if let Some(tile) = level.get_tile(player.position()) {
                    match tile.tile_type {
                        TileType::StairsUp => {
                            self.draw_wrapped_text("1: Go up stairs (<)", panel_x, line_y, normal_font_size, WHITE, panel_width);
                            line_y += line_height;
                        }
                        TileType::StairsDown => {
                            self.draw_wrapped_text("2: Go down stairs (>)", panel_x, line_y, normal_font_size, WHITE, panel_width);
                            line_y += line_height;
                        }
                        _ => {
                            // Show greyed out stair options when not on stairs
                            self.draw_wrapped_text("1: Go up stairs (<)", panel_x, line_y, normal_font_size, GRAY, panel_width);
                            line_y += line_height;
                            self.draw_wrapped_text("2: Go down stairs (>)", panel_x, line_y, normal_font_size, GRAY, panel_width);
                            line_y += line_height;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Renders the message area.
    fn render_messages(&self) -> ThatchResult<()> {
        // Responsive font sizes and spacing
        let scale_factor = (self.screen_width / 1024.0).max(0.7).min(1.3);
        let normal_font_size = 16.0 * scale_factor;
        let line_height = 18.0 * scale_factor;
        
        let message_area_height = 80.0 * scale_factor;
        let message_area_y = self.screen_height - message_area_height;
        let message_count = 3; // Show last 3 messages

        // Draw background for message area
        draw_rectangle(
            0.0,
            message_area_y - 10.0,
            self.screen_width,
            message_area_height + 10.0,
            Color::new(0.0, 0.0, 0.0, 0.8),
        );

        // Render messages
        let start_index = if self.messages.len() > message_count {
            self.messages.len() - message_count
        } else {
            0
        };

        for (i, message) in self.messages.iter().skip(start_index).enumerate() {
            let y = message_area_y + i as f32 * line_height;
            draw_text(message, 10.0, y, normal_font_size, WHITE);
        }

        Ok(())
    }

    /// Gets touch input from UI controls.
    ///
    /// Returns player input if a touch control was activated, None otherwise.
    pub fn get_touch_input(&self) -> Option<PlayerInput> {
        self.ui.render_touch_controls()
    }

    /// Adds a message to the message history.
    pub fn add_message(&mut self, message: String) {
        self.messages.push(message);

        // Keep only the most recent messages
        if self.messages.len() > self.max_messages {
            self.messages.remove(0);
        }
    }
}
