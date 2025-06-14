//! Integration tests for stair navigation and 3D dungeon functionality.

use thatch::{
    GameState, PlayerCharacter, Entity, ConcreteEntity, Position, TileType,
    StairDirection, UseStairsAction, ConcreteAction, Action, GenerationConfig,
    RoomCorridorGenerator, WorldGenerator,
};
use rand::{rngs::StdRng, SeedableRng};

/// Test stair navigation between floors using the 3D generation system.
#[test]
fn test_stair_navigation_3d_dungeon() {
    // Create a game state with complete 3D dungeon
    let seed = 98765;
    let mut game_state = GameState::new_with_complete_dungeon(seed)
        .expect("Failed to create 3D dungeon");
    
    // Create and place player on level 0
    let player_entity = ConcreteEntity::Player(PlayerCharacter::new("TestHero".to_string()));
    let player_id = player_entity.id();
    
    // Add player to game state
    game_state.add_entity(player_entity).expect("Failed to add player");
    game_state.set_player(player_id).expect("Failed to set player");
    
    // Get initial spawn position (should be on level 0)
    let initial_pos = game_state.world.current_level()
        .expect("No current level")
        .player_spawn;
    
    game_state.set_entity_position(player_id, initial_pos)
        .expect("Failed to set player position");
    
    // Verify we start on level 0
    assert_eq!(game_state.world.current_level_id, 0);
    
    // Find stairs down on level 0
    let stairs_down_pos = game_state.world.current_level()
        .expect("No current level")
        .stairs_down_position
        .expect("Level 0 should have stairs down");
    
    // Move player to stairs down
    game_state.set_entity_position(player_id, stairs_down_pos)
        .expect("Failed to move player to stairs");
    
    // Use stairs down
    let use_stairs_action = UseStairsAction::new(player_id, StairDirection::Down);
    let events = use_stairs_action.execute(&mut game_state)
        .expect("Failed to use stairs down");
    
    // Verify we moved to level 1
    assert_eq!(game_state.world.current_level_id, 1);
    assert!(!events.is_empty(), "Should have generated level change events");
    
    // Verify player position is at stairs up on level 1
    let level_1_stairs_up = game_state.world.current_level()
        .expect("No level 1")
        .stairs_up_position
        .expect("Level 1 should have stairs up");
    
    let player_pos = game_state.get_entity_position(player_id)
        .expect("Player should have position");
    
    assert_eq!(player_pos, level_1_stairs_up, "Player should be at stairs up on level 1");
    
    // Verify stairs alignment: level 0 stairs down should match level 1 stairs up
    assert_eq!(stairs_down_pos, level_1_stairs_up, "Stairs should be aligned between levels");
}

/// Test that stairs are properly aligned across all 26 floors.
#[test]
fn test_complete_stair_alignment() {
    let seed = 54321;
    let game_state = GameState::new_with_complete_dungeon(seed)
        .expect("Failed to create 3D dungeon");
    
    // Verify all 26 levels exist
    for level_id in 0..26 {
        assert!(game_state.world.get_level(level_id).is_some(), 
                "Level {} should exist", level_id);
    }
    
    // Verify stair alignment between consecutive levels
    for level_id in 0..25 {
        let current_level = game_state.world.get_level(level_id)
            .expect(&format!("Level {} should exist", level_id));
        let next_level = game_state.world.get_level(level_id + 1)
            .expect(&format!("Level {} should exist", level_id + 1));
        
        // Current level's down stairs should match next level's up stairs
        if let (Some(down_pos), Some(up_pos)) = 
            (current_level.stairs_down_position, next_level.stairs_up_position) {
            assert_eq!(down_pos, up_pos, 
                      "Stairs should align between levels {} and {}", level_id, level_id + 1);
        }
        
        // Verify level 0 has no up stairs
        if level_id == 0 {
            assert!(current_level.stairs_up_position.is_none(), 
                   "Level 0 should not have up stairs");
        } else {
            assert!(current_level.stairs_up_position.is_some(), 
                   "Level {} should have up stairs", level_id);
        }
        
        // Verify level 25 has no down stairs  
        if level_id == 24 {
            assert!(next_level.stairs_down_position.is_none(), 
                   "Level 25 should not have down stairs");
        } else {
            assert!(current_level.stairs_down_position.is_some(), 
                   "Level {} should have down stairs", level_id);
        }
    }
}

/// Test trying to go beyond level boundaries.
#[test]
fn test_stair_boundary_conditions() {
    let seed = 11111;
    let mut game_state = GameState::new_with_complete_dungeon(seed)
        .expect("Failed to create 3D dungeon");
    
    // Create player
    let player_entity = ConcreteEntity::Player(PlayerCharacter::new("TestHero".to_string()));
    let player_id = player_entity.id();
    
    game_state.add_entity(player_entity).expect("Failed to add player");
    game_state.set_player(player_id).expect("Failed to set player");
    
    // Test going up from level 0 (should trigger escape ending)
    let initial_pos = game_state.world.current_level()
        .expect("No current level")
        .player_spawn;
    game_state.set_entity_position(player_id, initial_pos)
        .expect("Failed to set player position");
    
    // Create fake stairs up on level 0 for testing
    let test_stairs_up_pos = Position::new(10, 10);
    game_state.set_entity_position(player_id, test_stairs_up_pos)
        .expect("Failed to move player");
    
    // Set the tile to stairs up temporarily
    if let Some(level) = game_state.world.current_level_mut() {
        level.set_tile(test_stairs_up_pos, thatch::Tile::new(TileType::StairsUp))
            .expect("Failed to set stairs up tile");
    }
    
    let use_stairs_action = UseStairsAction::new(player_id, StairDirection::Up);
    let result = use_stairs_action.execute(&mut game_state);
    
    // Should succeed but trigger escape ending
    assert!(result.is_ok(), "Using stairs up from level 0 should succeed");
    assert_eq!(game_state.completion_state, thatch::GameCompletionState::EscapedEarly);
    
    // Test going down from level 25 (should trigger win ending)
    let mut game_state_25 = GameState::new_with_complete_dungeon(seed + 1)
        .expect("Failed to create 3D dungeon");
    
    let player_entity_25 = ConcreteEntity::Player(PlayerCharacter::new("TestHero25".to_string()));
    let player_id_25 = player_entity_25.id();
    
    game_state_25.add_entity(player_entity_25).expect("Failed to add player");
    game_state_25.set_player(player_id_25).expect("Failed to set player");
    
    // Move to level 25
    game_state_25.world.change_level(25).expect("Failed to change to level 25");
    
    let level_25_pos = game_state_25.world.current_level()
        .expect("No level 25")
        .player_spawn;
    game_state_25.set_entity_position(player_id_25, level_25_pos)
        .expect("Failed to set player position on level 25");
    
    // Create fake stairs down on level 25 for testing
    let test_stairs_down_pos = Position::new(15, 15);
    game_state_25.set_entity_position(player_id_25, test_stairs_down_pos)
        .expect("Failed to move player");
    
    if let Some(level) = game_state_25.world.current_level_mut() {
        level.set_tile(test_stairs_down_pos, thatch::Tile::new(TileType::StairsDown))
            .expect("Failed to set stairs down tile");
    }
    
    let use_stairs_action_25 = UseStairsAction::new(player_id_25, StairDirection::Down);
    let result_25 = use_stairs_action_25.execute(&mut game_state_25);
    
    // Should succeed but trigger win ending
    assert!(result_25.is_ok(), "Using stairs down from level 25 should succeed");
    assert_eq!(game_state_25.completion_state, thatch::GameCompletionState::CompletedDungeon);
}

/// Test 3D generation produces valid levels with required features.
#[test]
fn test_3d_generation_validity() {
    let seed = 77777;
    let game_state = GameState::new_with_complete_dungeon(seed)
        .expect("Failed to create 3D dungeon");
    
    for level_id in 0..26 {
        let level = game_state.world.get_level(level_id)
            .expect(&format!("Level {} should exist", level_id));
        
        // Each level should have passable tiles
        let passable_count = level.tiles.iter()
            .flat_map(|row| row.iter())
            .filter(|tile| tile.tile_type.is_passable())
            .count();
        
        assert!(passable_count > 0, "Level {} should have passable tiles", level_id);
        
        // Verify spawn point is passable
        let spawn_tile = level.get_tile(level.player_spawn)
            .expect("Spawn position should be valid");
        assert!(spawn_tile.tile_type.is_passable(), 
               "Spawn position should be passable on level {}", level_id);
        
        // Verify appropriate stairs exist
        if level_id > 0 {
            assert!(level.stairs_up_position.is_some(), 
                   "Level {} should have up stairs", level_id);
        }
        if level_id < 25 {
            assert!(level.stairs_down_position.is_some(), 
                   "Level {} should have down stairs", level_id);
        }
    }
}

/// Test that WorldGenerator trait works correctly.
#[test]
fn test_world_generator_trait() {
    let seed = 33333;
    let config = GenerationConfig::new(seed);
    let mut rng = StdRng::seed_from_u64(seed);
    let generator = RoomCorridorGenerator::new();
    
    // Generate world using trait
    let world = generator.generate_world(&config, &mut rng)
        .expect("World generation should succeed");
    
    // Validate world using trait
    generator.validate_world(&world, &config)
        .expect("Generated world should be valid");
    
    // Verify world has 26 levels
    assert_eq!(world.levels.len(), 26, "World should have 26 levels");
    
    // Verify each level is valid
    for level_id in 0..26 {
        assert!(world.get_level(level_id).is_some(), "Level {} should exist", level_id);
    }
}