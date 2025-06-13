//! # Thatch Roguelike Main Entry Point
//!
//! Initializes the game state, sets up terminal rendering, and runs the main game loop.

use clap::Parser;
use std::time::{Duration, Instant};
use thatch::{
    Entity, GameState, GenerationConfig, Generator, PlayerCharacter, RoomCorridorGenerator,
    ThatchError, ThatchResult,
};
#[cfg(feature = "dev-tools")]
use tracing::{error, info, Level};
#[cfg(feature = "dev-tools")]
use tracing_subscriber;

#[cfg(not(feature = "dev-tools"))]
macro_rules! info {
    ($($arg:tt)*) => { println!($($arg)*) };
}

#[cfg(not(feature = "dev-tools"))]
macro_rules! error {
    ($($arg:tt)*) => { eprintln!($($arg)*) };
}

/// Command line arguments for the Thatch roguelike.
#[derive(Parser, Debug)]
#[command(name = "thatch")]
#[command(about = "A deep, complex roguelike with LLM-driven dungeon mastering")]
#[command(version)]
struct Args {
    /// Random seed for dungeon generation
    #[arg(short, long)]
    seed: Option<u64>,

    /// Enable development mode with debug tools
    #[arg(long)]
    dev_mode: bool,

    /// Enable AI player mode
    #[arg(long)]
    ai_player: bool,

    /// Start MCP server mode
    #[arg(long)]
    mcp_server: bool,

    /// Log level (error, warn, info, debug, trace)
    #[arg(long, default_value = "info")]
    log_level: String,
}

fn main() -> ThatchResult<()> {
    let args = Args::parse();

    // Initialize logging
    initialize_logging(&args.log_level)?;

    info!("Starting Thatch Roguelike v{}", thatch::VERSION);

    if args.mcp_server {
        #[cfg(feature = "mcp-server")]
        {
            info!("Starting in MCP server mode");
            return start_mcp_server().await;
        }
        #[cfg(not(feature = "mcp-server"))]
        {
            error!("MCP server feature not enabled. Rebuild with --features mcp-server");
            return Err(ThatchError::InvalidState(
                "MCP server not available".to_string(),
            ));
        }
    }

    if args.ai_player {
        info!("Starting in AI player mode");
        return run_ai_player_mode(&args);
    }

    // Normal game mode
    info!("Starting in normal game mode");
    run_game(&args)
}

/// Initializes the logging system based on the specified log level.
fn initialize_logging(log_level: &str) -> ThatchResult<()> {
    #[cfg(feature = "dev-tools")]
    {
        let level = match log_level.to_lowercase().as_str() {
            "error" => Level::ERROR,
            "warn" => Level::WARN,
            "info" => Level::INFO,
            "debug" => Level::DEBUG,
            "trace" => Level::TRACE,
            _ => Level::INFO,
        };

        tracing_subscriber::fmt()
            .with_max_level(level)
            .with_target(false)
            .init();
    }

    #[cfg(not(feature = "dev-tools"))]
    {
        let _ = log_level; // Suppress unused variable warning
        println!("Logging initialized (basic mode)");
    }

    Ok(())
}

/// Runs the main game loop with terminal interface.
fn run_game(args: &Args) -> ThatchResult<()> {
    // Initialize the display system
    info!("Initializing terminal display");
    let mut display = thatch::Display::new()?;

    // Initialize input handler
    let input_handler = thatch::InputHandler::new();

    let result = run_game_loop(args, &mut display, &input_handler);

    // Cleanup terminal
    display.cleanup()?;

    result
}

/// Main game loop implementation.
fn run_game_loop(
    args: &Args,
    display: &mut thatch::Display,
    input_handler: &thatch::InputHandler,
) -> ThatchResult<()> {
    // Generate a proper dungeon level
    let seed = args.seed.unwrap_or(12345);

    info!("Generating dungeon level with seed: {}", seed);

    // Create generation configuration
    let config = GenerationConfig::for_testing(seed);
    let generator = RoomCorridorGenerator::for_testing();
    let mut rng = thatch::generation::utils::create_rng(&config);

    // Generate the level
    let level = generator.generate(&config, &mut rng)?;

    // Initialize game state
    info!("Initializing game state");
    let mut game_state = GameState::new_with_level(level, seed)?;

    // Create and place player at the spawn point
    let player_pos = if let Some(level) = game_state.world.current_level() {
        level.player_spawn
    } else {
        return Err(ThatchError::InvalidState("No current level".to_string()));
    };
    let player = PlayerCharacter::new("Player".to_string(), player_pos);
    let player_id = game_state.add_entity(player.into())?;
    game_state.set_player_id(player_id);

    // Initialize player visibility
    if let Some(player) = game_state.get_player() {
        game_state.update_player_visibility(player.position())?;
    }

    info!("Player created and placed at {:?}", player_pos);

    display.add_message("Welcome to Thatch Roguelike!".to_string());
    display.add_message("Use hjkl or arrow keys to move, q to quit".to_string());

    // Frame rate limiting
    let target_frame_time = Duration::from_millis(1000 / 60); // 60 FPS

    // Main game loop
    loop {
        let frame_start = Instant::now();

        // Render the game
        display.render_game(&game_state)?;

        // Process input (non-blocking)
        if let Some(input) = input_handler.wait_for_input()? {
            match input {
                thatch::PlayerInput::Quit => {
                    info!("Player quit the game");
                    break;
                }

                thatch::PlayerInput::Resize {
                    width: _,
                    height: _,
                } => {
                    display.update_size()?;
                    continue;
                }

                thatch::PlayerInput::Help => {
                    display.add_message("Help: hjkl/arrows=move, q=quit, .=wait".to_string());
                    continue;
                }

                _ => {
                    // Convert input to action and execute it
                    if let Some(action) =
                        input_handler.input_to_action(input.clone(), &game_state)?
                    {
                        // Execute the action
                        match action.execute(&mut game_state) {
                            Ok(events) => {
                                // Process any resulting events
                                for event in &events {
                                    let response_events = game_state.process_event(event)?;

                                    // Display any messages from events
                                    for response_event in response_events {
                                        if let thatch::GameEvent::Message { text, .. } =
                                            response_event
                                        {
                                            display.add_message(text);
                                        }
                                    }
                                }

                                // Advance the turn
                                game_state.advance_turn()?;
                            }
                            Err(e) => {
                                display.add_message(format!("Invalid action: {}", e));
                            }
                        }
                    }
                }
            }
        }

        // Frame rate limiting
        let frame_elapsed = frame_start.elapsed();
        if frame_elapsed < target_frame_time {
            std::thread::sleep(target_frame_time - frame_elapsed);
        }
    }

    info!("Game loop ended");
    Ok(())
}

/// Runs AI player mode for testing and demonstration.
fn run_ai_player_mode(_args: &Args) -> ThatchResult<()> {
    info!("AI player mode not yet implemented");
    // TODO: Implement AI player
    Ok(())
}

/// Starts the MCP server for external control.
#[cfg(feature = "mcp-server")]
async fn start_mcp_server() -> ThatchResult<()> {
    info!("MCP server mode not yet implemented");
    // TODO: Implement MCP server
    Ok(())
}
