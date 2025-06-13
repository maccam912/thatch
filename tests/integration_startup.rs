//! Integration test to ensure the application can start up without errors.

use thatch::{GameState, Level, PlayerCharacter, Position, ThatchResult, TileType};

#[test]
fn test_basic_startup() -> ThatchResult<()> {
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

    // Verify the game state is properly initialized
    assert!(game_state.player_id.is_some());
    assert_eq!(game_state.turn_number, 0);
    assert!(game_state.entities.contains_key(&player_id));

    // Verify the player is in a valid position
    let level = game_state.world.current_level().unwrap();
    if let Some(tile) = level.get_tile(player_pos) {
        assert_eq!(tile.tile_type, TileType::Floor);
    }

    Ok(())
}

#[test]
fn test_player_can_be_created() {
    let player = PlayerCharacter::new("Hero".to_string(), Position::new(5, 5));
    assert_eq!(player.name, "Hero");
    assert_eq!(player.position, Position::new(5, 5));

    // Test that player implements Entity trait
    use thatch::Entity;
    assert!(player.is_alive());
    assert_eq!(player.position(), Position::new(5, 5));
}
