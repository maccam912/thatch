//! Integration test for terminal rendering system functionality.

use thatch::{
    Direction, Entity, GameState, InputHandler, Level, PlayerCharacter, PlayerInput, Position,
    ThatchResult, TileType,
};

#[test]
fn test_game_state_with_player_visibility() -> ThatchResult<()> {
    // Create a simple test level
    let mut level = Level::new(0, 10, 10);

    // Fill with floor tiles except borders (walls)
    for y in 0..10 {
        for x in 0..10 {
            let pos = Position::new(x as i32, y as i32);
            if let Some(tile) = level.get_tile_mut(pos) {
                if x == 0 || x == 9 || y == 0 || y == 9 {
                    tile.tile_type = TileType::Wall;
                } else {
                    tile.tile_type = TileType::Floor;
                }
            }
        }
    }

    // Create game state with the level
    let mut game_state = GameState::new_with_level(level, 12345)?;

    // Find a starting position and create player
    let player_pos = game_state.find_starting_position()?;
    let player = PlayerCharacter::new("TestPlayer".to_string(), player_pos);
    let player_id = game_state.add_entity(player.into())?;
    game_state.set_player_id(player_id);

    // Update player visibility
    if let Some(player) = game_state.get_player() {
        game_state.update_player_visibility(player.position())?;
    }

    // Verify the game state is properly initialized
    assert!(game_state.player_id.is_some());
    assert_eq!(game_state.turn_number, 0);
    assert!(game_state.entities.contains_key(&player_id));

    // Verify some tiles are now visible around the player
    let level = game_state.world.current_level().unwrap();
    let player_tile = level.get_tile(player_pos).unwrap();
    assert!(player_tile.is_visible());
    assert!(player_tile.is_explored());

    Ok(())
}

#[test]
fn test_input_handler_movement_conversion() -> ThatchResult<()> {
    let input_handler = InputHandler::new();

    // Create a simple game state for testing
    let level = Level::new(0, 10, 10);
    let mut game_state = GameState::new_with_level(level, 12345)?;
    let player_pos = Position::new(5, 5);
    let player = PlayerCharacter::new("TestPlayer".to_string(), player_pos);
    let player_id = game_state.add_entity(player.into())?;
    game_state.set_player_id(player_id);

    // Test movement input conversion
    let move_input = PlayerInput::Move(Position::new(1, 0)); // East
    let action = input_handler.input_to_action(move_input, &game_state)?;

    assert!(action.is_some());

    // Test wait input conversion
    let wait_input = PlayerInput::Wait;
    let action = input_handler.input_to_action(wait_input, &game_state)?;

    assert!(action.is_some());

    Ok(())
}

#[test]
fn test_direction_conversion() {
    // Test position delta to direction conversion
    assert_eq!(
        Direction::from_delta(Position::new(0, -1)),
        Some(Direction::North)
    );
    assert_eq!(
        Direction::from_delta(Position::new(0, 1)),
        Some(Direction::South)
    );
    assert_eq!(
        Direction::from_delta(Position::new(1, 0)),
        Some(Direction::East)
    );
    assert_eq!(
        Direction::from_delta(Position::new(-1, 0)),
        Some(Direction::West)
    );

    // Test diagonal directions
    assert_eq!(
        Direction::from_delta(Position::new(1, -1)),
        Some(Direction::Northeast)
    );
    assert_eq!(
        Direction::from_delta(Position::new(-1, -1)),
        Some(Direction::Northwest)
    );
    assert_eq!(
        Direction::from_delta(Position::new(1, 1)),
        Some(Direction::Southeast)
    );
    assert_eq!(
        Direction::from_delta(Position::new(-1, 1)),
        Some(Direction::Southwest)
    );

    // Test invalid direction
    assert_eq!(Direction::from_delta(Position::new(2, 0)), None);
    assert_eq!(Direction::from_delta(Position::new(0, 0)), None);
}

#[test]
fn test_tile_visibility_system() -> ThatchResult<()> {
    let mut level = Level::new(0, 5, 5);

    // All tiles start as walls and unexplored
    for y in 0..5 {
        for x in 0..5 {
            let pos = Position::new(x as i32, y as i32);
            if let Some(tile) = level.get_tile(pos) {
                assert!(!tile.is_visible());
                assert!(!tile.is_explored());
            }
        }
    }

    // Set center tile to visible
    let center_pos = Position::new(2, 2);
    if let Some(tile) = level.get_tile_mut(center_pos) {
        tile.set_visible(true);
    }

    // Verify visibility
    if let Some(tile) = level.get_tile(center_pos) {
        assert!(tile.is_visible());
        assert!(tile.is_explored()); // Should be explored when set visible
    }

    // Set tile to not visible but should remain explored
    if let Some(tile) = level.get_tile_mut(center_pos) {
        tile.set_visible(false);
    }

    if let Some(tile) = level.get_tile(center_pos) {
        assert!(!tile.is_visible());
        assert!(tile.is_explored()); // Should remain explored
    }

    Ok(())
}

#[test]
fn test_player_movement_action() -> ThatchResult<()> {
    // Create a simple level with floor tiles
    let mut level = Level::new(0, 10, 10);
    for y in 1..9 {
        for x in 1..9 {
            let pos = Position::new(x as i32, y as i32);
            if let Some(tile) = level.get_tile_mut(pos) {
                tile.tile_type = TileType::Floor;
            }
        }
    }

    let mut game_state = GameState::new_with_level(level, 12345)?;
    let start_pos = Position::new(5, 5);
    let player = PlayerCharacter::new("TestPlayer".to_string(), start_pos);
    let player_id = game_state.add_entity(player.into())?;
    game_state.set_player_id(player_id);

    // Test movement action through input system
    let input_handler = InputHandler::new();
    let move_east = PlayerInput::Move(Position::new(1, 0));

    // Debug: check that the player is properly set
    assert!(game_state.player_id.is_some());
    assert_eq!(game_state.player_id.unwrap(), player_id);
    assert!(game_state.get_entity_position(player_id).is_some());

    if let Some(action) = input_handler.input_to_action(move_east, &game_state)? {
        // Execute the movement action
        let events = action.execute(&mut game_state)?;

        // Verify that a move event was generated
        assert_eq!(events.len(), 1);
        if let thatch::GameEvent::EntityMoved {
            entity_id,
            from,
            to,
        } = &events[0]
        {
            assert_eq!(*entity_id, player_id);
            assert_eq!(*from, start_pos);
            assert_eq!(*to, Position::new(6, 5)); // Moved east
        } else {
            panic!("Expected EntityMoved event");
        }

        // Verify player position was updated
        if let Some(player) = game_state.get_player() {
            assert_eq!(player.position(), Position::new(6, 5));
        }
    }

    Ok(())
}
