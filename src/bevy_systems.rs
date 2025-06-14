//! # Bevy Systems for Thatch Roguelike
//!
//! Contains all the Bevy systems for rendering, input handling, and game logic.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use crate::game::*;

/// Setup the game camera
pub fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera2dBundle::default(),
        GameCamera,
    ));
    
    commands.insert_resource(Viewport::default());
    commands.insert_resource(MessageLog::default());
    commands.insert_resource(InputState::default());
}

/// Initialize the game world and entities
pub fn setup_game(
    mut commands: Commands,
    config: Res<GameConfig>,
    mut message_log: ResMut<MessageLog>,
) {
    info!("Generating dungeon level with seed: {}", config.seed);
    
    // Generate the dungeon level
    let generation_config = GenerationConfig::for_testing(config.seed);
    let generator = RoomCorridorGenerator::for_testing();
    let mut rng = thatch::generation::utils::create_rng(&generation_config);
    
    let level = match generator.generate(&generation_config, &mut rng) {
        Ok(level) => level,
        Err(e) => {
            error!("Failed to generate level: {}", e);
            return;
        }
    };

    let player_spawn = level.player_spawn;
    
    // Create game state
    let mut game_state = match GameState::new_with_level(level, config.seed) {
        Ok(state) => state,
        Err(e) => {
            error!("Failed to create game state: {}", e);
            return;
        }
    };
    
    // Create and place player
    let player = PlayerCharacter::new("Player".to_string(), player_spawn);
    let player_id = match game_state.add_entity(player.into()) {
        Ok(id) => id,
        Err(e) => {
            error!("Failed to add player entity: {}", e);
            return;
        }
    };
    game_state.set_player_id(player_id);
    
    // Initialize player visibility
    if let Some(player) = game_state.get_player() {
        if let Err(e) = game_state.update_player_visibility(player.position()) {
            error!("Failed to update player visibility: {}", e);
        }
    }
    
    // Wrap game state for Bevy
    let bevy_game_state = BevyGameState {
        inner: game_state,
    };
    commands.insert_resource(bevy_game_state);
    
    // Spawn player entity in Bevy ECS
    commands.spawn((
        Player::default(),
        WorldPosition::from(player_spawn),
        SpatialBundle::default(),
    ));
    
    // Add welcome messages
    message_log.add_message("Welcome to Thatch Roguelike!".to_string());
    message_log.add_message("Use WASD or arrow keys to move, Q to quit".to_string());
    message_log.add_message("Stand on stairs and press < or > to use them".to_string());
    
    info!("Game initialized with seed: {}", config.seed);
}

/// Handle keyboard input for player actions
pub fn handle_input(
    keys: Res<Input<KeyCode>>,
    mut input_state: ResMut<InputState>,
    mut game_state: ResMut<BevyGameState>,
    mut message_log: ResMut<MessageLog>,
    mut player_query: Query<&mut WorldPosition, With<Player>>,
    mut app_exit_events: EventWriter<AppExit>,
) {
    // Only process input if no action is pending (turn-based)
    if input_state.pending_action.is_some() {
        return;
    }
    
    let mut action = None;
    
    // Movement keys
    if keys.just_pressed(KeyCode::W) || keys.just_pressed(KeyCode::Up) {
        action = Some(PlayerAction::Move { dx: 0, dy: 1 });
    } else if keys.just_pressed(KeyCode::S) || keys.just_pressed(KeyCode::Down) {
        action = Some(PlayerAction::Move { dx: 0, dy: -1 });
    } else if keys.just_pressed(KeyCode::A) || keys.just_pressed(KeyCode::Left) {
        action = Some(PlayerAction::Move { dx: -1, dy: 0 });
    } else if keys.just_pressed(KeyCode::D) || keys.just_pressed(KeyCode::Right) {
        action = Some(PlayerAction::Move { dx: 1, dy: 0 });
    }
    
    // Other actions
    else if keys.just_pressed(KeyCode::Period) {
        action = Some(PlayerAction::Wait);
    } else if keys.just_pressed(KeyCode::Comma) {
        action = Some(PlayerAction::GoUpstairs);
    } else if keys.just_pressed(KeyCode::Period) && keys.pressed(KeyCode::ShiftLeft) {
        action = Some(PlayerAction::GoDownstairs);
    } else if keys.just_pressed(KeyCode::Q) {
        app_exit_events.send(AppExit);
        return;
    }
    
    // Process the action if we have one
    if let Some(player_action) = action {
        if let Ok(mut player_pos) = player_query.get_single_mut() {
            match &player_action {
                PlayerAction::Move { dx, dy } => {
                    let new_pos = Position::new(player_pos.x + dx, player_pos.y + dy);
                    
                    // Check if the move is valid
                    if let Some(level) = game_state.inner.world.current_level() {
                        if let Some(tile) = level.get_tile(new_pos) {
                            match tile.tile_type {
                                TileType::Wall => {
                                    // Can't move into walls - no message to reduce spam
                                }
                                TileType::Floor | TileType::Door { is_open: true } | 
                                TileType::StairsUp | TileType::StairsDown => {
                                    player_pos.x = new_pos.x;
                                    player_pos.y = new_pos.y;
                                    
                                    // Update thatch game state
                                    if let Some(player) = game_state.inner.get_player_mut() {
                                        player.set_position(new_pos);
                                    }
                                    
                                    if let Err(e) = game_state.inner.advance_turn() {
                                        error!("Failed to advance turn: {}", e);
                                    }
                                    
                                    // Update visibility around player
                                    if let Err(e) = game_state.inner.update_player_visibility(new_pos) {
                                        error!("Failed to update visibility: {}", e);
                                    }
                                }
                                TileType::Door { is_open: false } => {
                                    message_log.add_message("The door is closed.".to_string());
                                }
                                TileType::Water => {
                                    message_log.add_message("You can't swim!".to_string());
                                }
                                TileType::Special { .. } => {
                                    player_pos.x = new_pos.x;
                                    player_pos.y = new_pos.y;
                                    
                                    if let Some(player) = game_state.inner.get_player_mut() {
                                        player.set_position(new_pos);
                                    }
                                    
                                    if let Err(e) = game_state.inner.advance_turn() {
                                        error!("Failed to advance turn: {}", e);
                                    }
                                    
                                    message_log.add_message("You step on something special...".to_string());
                                }
                            }
                        }
                    }
                }
                PlayerAction::GoUpstairs => {
                    let current_pos = Position::new(player_pos.x, player_pos.y);
                    if let Some(level) = game_state.inner.world.current_level() {
                        if let Some(tile) = level.get_tile(current_pos) {
                            if matches!(tile.tile_type, TileType::StairsUp) {
                                match game_state.inner.use_stairs(thatch::StairDirection::Up) {
                                    Ok(true) => {
                                        message_log.add_message("You ascend the stairs.".to_string());
                                    }
                                    Ok(false) => {
                                        message_log.add_message("You escape the dungeon!".to_string());
                                        // Will be handled by check_completion system
                                    }
                                    Err(e) => {
                                        error!("Failed to use stairs: {}", e);
                                        message_log.add_message("You can't use these stairs right now.".to_string());
                                    }
                                }
                            } else {
                                message_log.add_message("There are no stairs up here.".to_string());
                            }
                        }
                    }
                }
                PlayerAction::GoDownstairs => {
                    let current_pos = Position::new(player_pos.x, player_pos.y);
                    if let Some(level) = game_state.inner.world.current_level() {
                        if let Some(tile) = level.get_tile(current_pos) {
                            if matches!(tile.tile_type, TileType::StairsDown) {
                                match game_state.inner.use_stairs(thatch::StairDirection::Down) {
                                    Ok(true) => {
                                        message_log.add_message("You descend the stairs.".to_string());
                                    }
                                    Ok(false) => {
                                        message_log.add_message("You've conquered the deepest depths!".to_string());
                                        // Will be handled by check_completion system
                                    }
                                    Err(e) => {
                                        error!("Failed to use stairs: {}", e);
                                        message_log.add_message("You can't use these stairs right now.".to_string());
                                    }
                                }
                            } else {
                                message_log.add_message("There are no stairs down here.".to_string());
                            }
                        }
                    }
                }
                PlayerAction::Wait => {
                    message_log.add_message("You wait...".to_string());
                    if let Err(e) = game_state.inner.advance_turn() {
                        error!("Failed to advance turn: {}", e);
                    }
                }
                _ => {}
            }
        }
        
        input_state.pending_action = None;
        input_state.last_input_time = Some(std::time::Instant::now());
    }
}

/// Update camera to follow player
pub fn update_camera(
    mut camera_query: Query<&mut Transform, (With<GameCamera>, Without<Player>)>,
    player_query: Query<&WorldPosition, With<Player>>,
    mut viewport: ResMut<Viewport>,
) {
    if let Ok(player_pos) = player_query.get_single() {
        viewport.center_x = player_pos.x as f32 * TILE_SIZE;
        viewport.center_y = player_pos.y as f32 * TILE_SIZE;
        
        if let Ok(mut camera_transform) = camera_query.get_single_mut() {
            camera_transform.translation.x = viewport.center_x;
            camera_transform.translation.y = viewport.center_y;
        }
    }
}

/// Render all tiles in the current level
pub fn render_tiles(
    mut commands: Commands,
    game_state: Res<BevyGameState>,
    viewport: Res<Viewport>,
    tile_query: Query<Entity, With<TileRenderer>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
) {
    // Clear existing tile entities
    for entity in tile_query.iter() {
        commands.entity(entity).despawn();
    }
    
    let window = window_query.get_single().unwrap();
    let tiles_per_screen_x = (window.width() / TILE_SIZE) as i32 + 2;
    let tiles_per_screen_y = (window.height() / TILE_SIZE) as i32 + 2;
    
    let start_x = (viewport.center_x / TILE_SIZE) as i32 - tiles_per_screen_x / 2;
    let start_y = (viewport.center_y / TILE_SIZE) as i32 - tiles_per_screen_y / 2;
    
    if let Some(level) = game_state.inner.world.current_level() {
        for y in start_y..(start_y + tiles_per_screen_y) {
            for x in start_x..(start_x + tiles_per_screen_x) {
                let pos = Position::new(x, y);
                
                if let Some(tile) = level.get_tile(pos) {
                    let world_x = x as f32 * TILE_SIZE;
                    let world_y = y as f32 * TILE_SIZE;
                    
                    let (color, character) = if tile.is_visible() {
                        get_tile_appearance(&tile.tile_type)
                    } else if tile.is_explored() {
                        let (_, ch) = get_tile_appearance(&tile.tile_type);
                        (Colors::EXPLORED, ch)
                    } else {
                        continue; // Don't render unexplored tiles
                    };
                    
                    commands.spawn((
                        SpriteBundle {
                            sprite: Sprite {
                                color,
                                custom_size: Some(Vec2::new(TILE_SIZE, TILE_SIZE)),
                                ..default()
                            },
                            transform: Transform::from_translation(Vec3::new(world_x, world_y, 0.0)),
                            ..default()
                        },
                        TileRenderer {
                            tile_type: tile.tile_type.clone(),
                            is_visible: tile.is_visible(),
                            is_explored: tile.is_explored(),
                        },
                    ));
                    
                    // Add character overlay for tiles that need it
                    if character != ' ' {
                        commands.spawn((
                            Text2dBundle {
                                text: Text::from_section(
                                    character.to_string(),
                                    TextStyle {
                                        font_size: TILE_SIZE * 0.8,
                                        color: Color::WHITE,
                                        ..default()
                                    },
                                ),
                                transform: Transform::from_translation(Vec3::new(world_x, world_y, 1.0)),
                                ..default()
                            },
                            TileRenderer {
                                tile_type: tile.tile_type.clone(),
                                is_visible: tile.is_visible(),
                                is_explored: tile.is_explored(),
                            },
                        ));
                    }
                }
            }
        }
    }
}

/// Render player and other entities
pub fn render_entities(
    mut commands: Commands,
    player_query: Query<&WorldPosition, With<Player>>,
    entity_query: Query<Entity, (With<Player>, Without<TileRenderer>)>,
) {
    // Clear existing player sprite
    for entity in entity_query.iter() {
        if let Ok(children) = commands.entity(entity).get::<Children>() {
            for child in children.iter() {
                commands.entity(*child).despawn();
            }
        }
    }
    
    if let Ok(player_pos) = player_query.get_single() {
        let world_x = player_pos.x as f32 * TILE_SIZE;
        let world_y = player_pos.y as f32 * TILE_SIZE;
        
        commands.spawn((
            Text2dBundle {
                text: Text::from_section(
                    "@",
                    TextStyle {
                        font_size: TILE_SIZE * 0.8,
                        color: Colors::PLAYER,
                        ..default()
                    },
                ),
                transform: Transform::from_translation(Vec3::new(world_x, world_y, 2.0)),
                ..default()
            },
            Player::default(),
        ));
    }
}

/// Render UI elements (stats, messages, etc.)
pub fn render_ui(
    mut commands: Commands,
    game_state: Res<BevyGameState>,
    message_log: Res<MessageLog>,
    player_query: Query<&Player>,
    ui_query: Query<Entity, With<UiElement>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
) {
    // Clear existing UI
    for entity in ui_query.iter() {
        commands.entity(entity).despawn_recursive();
    }
    
    let window = window_query.get_single().unwrap();
    
    // Create UI root
    commands.spawn((
        NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            background_color: Color::NONE.into(),
            ..default()
        },
        UiElement,
    )).with_children(|parent| {
        // Right panel for stats
        parent.spawn(NodeBundle {
            style: Style {
                width: Val::Px(UI_PANEL_WIDTH),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                right: Val::Px(0.0),
                top: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            background_color: Color::rgba(0.0, 0.0, 0.0, 0.8).into(),
            ..default()
        }).with_children(|panel| {
            // Title
            panel.spawn(TextBundle::from_section(
                "THATCH ROGUELIKE",
                TextStyle {
                    font_size: 20.0,
                    color: Color::WHITE,
                    ..default()
                },
            ));
            
            // Player stats
            if let Some(player) = game_state.inner.get_player() {
                panel.spawn(TextBundle::from_section(
                    format!("Player: {}", player.name()),
                    TextStyle {
                        font_size: 16.0,
                        color: Colors::PLAYER,
                        ..default()
                    },
                ));
                
                if let ConcreteEntity::Player(player_char) = player {
                    panel.spawn(TextBundle::from_section(
                        format!("Health: {}/{}", player_char.stats.health, player_char.stats.max_health),
                        TextStyle {
                            font_size: 14.0,
                            color: Color::WHITE,
                            ..default()
                        },
                    ));
                    
                    panel.spawn(TextBundle::from_section(
                        format!("Mana: {}/{}", player_char.stats.mana, player_char.stats.max_mana),
                        TextStyle {
                            font_size: 14.0,
                            color: Color::WHITE,
                            ..default()
                        },
                    ));
                    
                    panel.spawn(TextBundle::from_section(
                        format!("Level: {}", player_char.stats.level),
                        TextStyle {
                            font_size: 14.0,
                            color: Color::WHITE,
                            ..default()
                        },
                    ));
                }
            }
            
            // Game info
            panel.spawn(TextBundle::from_section(
                "Game Info:",
                TextStyle {
                    font_size: 16.0,
                    color: Colors::STAIRS_UP,
                    ..default()
                },
            ));
            
            let time_info = game_state.inner.get_game_time_info();
            panel.spawn(TextBundle::from_section(
                format!("Turn: {}", time_info.turn_number),
                TextStyle {
                    font_size: 14.0,
                    color: Color::WHITE,
                    ..default()
                },
            ));
            
            // Controls
            panel.spawn(TextBundle::from_section(
                "Controls:",
                TextStyle {
                    font_size: 16.0,
                    color: Color::GREEN,
                    ..default()
                },
            ));
            
            let controls = [
                "WASD/Arrows: Move",
                "<: Go up stairs",
                ">: Go down stairs",
                "Q: Quit",
            ];
            
            for control in &controls {
                panel.spawn(TextBundle::from_section(
                    *control,
                    TextStyle {
                        font_size: 12.0,
                        color: Color::WHITE,
                        ..default()
                    },
                ));
            }
        });
        
        // Bottom panel for messages
        parent.spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Px(100.0),
                position_type: PositionType::Absolute,
                bottom: Val::Px(0.0),
                left: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            background_color: Color::rgba(0.0, 0.0, 0.0, 0.8).into(),
            ..default()
        }).with_children(|panel| {
            // Show last few messages
            let start_index = if message_log.messages.len() > 3 {
                message_log.messages.len() - 3
            } else {
                0
            };
            
            for message in message_log.messages.iter().skip(start_index) {
                panel.spawn(TextBundle::from_section(
                    message,
                    TextStyle {
                        font_size: 14.0,
                        color: Color::WHITE,
                        ..default()
                    },
                ));
            }
        });
    });
}

/// Check for game completion conditions
pub fn check_completion(
    game_state: Res<BevyGameState>,
    mut app_exit_events: EventWriter<AppExit>,
) {
    match game_state.inner.get_completion_state() {
        GameCompletionState::EscapedEarly => {
            info!("Player escaped early!");
            app_exit_events.send(AppExit);
        }
        GameCompletionState::CompletedDungeon => {
            info!("Player completed the dungeon!");
            app_exit_events.send(AppExit);
        }
        GameCompletionState::PlayerDied => {
            info!("Player died!");
            app_exit_events.send(AppExit);
        }
        GameCompletionState::Playing => {
            // Continue playing
        }
    }
}

/// Helper function to get tile appearance
fn get_tile_appearance(tile_type: &TileType) -> (Color, char) {
    match tile_type {
        TileType::Wall => (Colors::WALL, '#'),
        TileType::Floor => (Colors::FLOOR, '.'),
        TileType::Door { is_open } => {
            if *is_open {
                (Colors::DOOR_OPEN, '\'')
            } else {
                (Colors::DOOR_CLOSED, '+')
            }
        }
        TileType::StairsUp => (Colors::STAIRS_UP, '<'),
        TileType::StairsDown => (Colors::STAIRS_DOWN, '>'),
        TileType::Water => (Colors::WATER, '~'),
        TileType::Special { .. } => (Colors::SPECIAL, '*'),
    }
}