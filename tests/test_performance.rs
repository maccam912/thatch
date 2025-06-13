//! Performance tests for the rendering system

use std::time::Instant;
use thatch::{Entity, GameState, Level, PlayerCharacter, Position, ThatchResult, TileType};

#[test]
fn test_frame_buffer_performance() -> ThatchResult<()> {
    // Create a larger test level for performance testing
    let mut level = Level::new(0, 50, 50);

    // Fill with a pattern of floor and wall tiles
    for y in 0..50 {
        for x in 0..50 {
            let pos = Position::new(x as i32, y as i32);
            if let Some(tile) = level.get_tile_mut(pos) {
                if x % 2 == 0 || y % 2 == 0 {
                    tile.tile_type = TileType::Wall;
                } else {
                    tile.tile_type = TileType::Floor;
                }
                // Set some tiles as visible for realistic rendering
                if x > 20 && x < 30 && y > 20 && y < 30 {
                    tile.set_visible(true);
                }
            }
        }
    }

    // Create game state
    let mut game_state = GameState::new_with_level(level, 12345)?;
    let player_pos = Position::new(25, 25);
    let player = PlayerCharacter::new("TestPlayer".to_string(), player_pos);
    let player_id = game_state.add_entity(player.into())?;
    game_state.set_player_id(player_id);

    // Update player visibility
    game_state.update_player_visibility(player_pos)?;

    // Benchmark frame buffer creation (simulating rendering without terminal)
    let start = Instant::now();
    let iterations = 100;

    for _ in 0..iterations {
        // Simulate the frame buffer operations that would happen in render_game
        let _current_player_pos = game_state.get_player().map(|p| p.position());

        // Simulate creating a frame buffer (this is the expensive part)
        let width = 80;
        let height = 24;
        let _frame_buffer = vec![vec!['.' as char; width]; height];

        // Simulate checking game state (lightweight operations)
        let _level = game_state.world.current_level();
        let _time_info = game_state.get_game_time_info();
    }

    let elapsed = start.elapsed();
    let avg_frame_time = elapsed / iterations;

    println!("Average frame processing time: {:?}", avg_frame_time);
    println!(
        "Theoretical max FPS: {:.1}",
        1.0 / avg_frame_time.as_secs_f64()
    );

    // Assert that frame processing is fast enough for 30+ FPS
    assert!(
        avg_frame_time.as_millis() < 33,
        "Frame processing too slow: {:?}",
        avg_frame_time
    );

    Ok(())
}

#[test]
fn test_visibility_update_performance() -> ThatchResult<()> {
    // Create a test level
    let mut level = Level::new(0, 100, 100);

    // Fill with floor tiles
    for y in 1..99 {
        for x in 1..99 {
            let pos = Position::new(x as i32, y as i32);
            if let Some(tile) = level.get_tile_mut(pos) {
                tile.tile_type = TileType::Floor;
            }
        }
    }

    let mut game_state = GameState::new_with_level(level, 12345)?;
    let player_pos = Position::new(50, 50);
    let player = PlayerCharacter::new("TestPlayer".to_string(), player_pos);
    let player_id = game_state.add_entity(player.into())?;
    game_state.set_player_id(player_id);

    // Benchmark visibility updates
    let start = Instant::now();
    let iterations = 50;

    for i in 0..iterations {
        // Move player around and update visibility
        let new_pos = Position::new(50 + (i % 10) as i32, 50 + (i / 10) as i32);
        game_state.set_entity_position(player_id, new_pos)?;
        game_state.update_player_visibility(new_pos)?;
    }

    let elapsed = start.elapsed();
    let avg_update_time = elapsed / iterations;

    println!("Average visibility update time: {:?}", avg_update_time);

    // Assert that visibility updates are fast enough
    assert!(
        avg_update_time.as_millis() < 5,
        "Visibility updates too slow: {:?}",
        avg_update_time
    );

    Ok(())
}

#[test]
fn test_game_state_operations_performance() -> ThatchResult<()> {
    // Create a test level
    let level = Level::new(0, 20, 20);
    let mut game_state = GameState::new_with_level(level, 12345)?;

    let player_pos = Position::new(10, 10);
    let player = PlayerCharacter::new("TestPlayer".to_string(), player_pos);
    let player_id = game_state.add_entity(player.into())?;
    game_state.set_player_id(player_id);

    // Benchmark common game operations
    let start = Instant::now();
    let iterations = 1000;

    for _ in 0..iterations {
        // Common operations that happen every frame
        let _player = game_state.get_player();
        let _player_pos = game_state.get_entity_position(player_id);
        let _entities_at_pos = game_state.get_entities_at_position(player_pos);
        let _level = game_state.world.current_level();
        let _time_info = game_state.get_game_time_info();

        // Simulate turn advancement occasionally
        if iterations % 100 == 0 {
            let _ = game_state.advance_turn();
        }
    }

    let elapsed = start.elapsed();
    let avg_operation_time = elapsed / iterations;

    println!("Average game operation time: {:?}", avg_operation_time);

    // Assert that game operations are very fast
    assert!(
        avg_operation_time.as_micros() < 100,
        "Game operations too slow: {:?}",
        avg_operation_time
    );

    Ok(())
}
