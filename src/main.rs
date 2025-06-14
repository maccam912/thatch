//! # Thatch Roguelike Main Entry Point
//!
//! Initializes the game state, sets up macroquad rendering, and runs the main game loop.

use clap::Parser;
use macroquad::prelude::*;
use thatch::{Entity, GameState, PlayerCharacter, SceneManager, ThatchError, ThatchResult};
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
#[clap(name = "thatch")]
#[clap(about = "A deep, complex roguelike with LLM-driven dungeon mastering")]
#[clap(version)]
struct Args {
    /// Random seed for dungeon generation
    #[clap(short, long)]
    seed: Option<u64>,

    /// Enable development mode with debug tools
    #[clap(long)]
    dev_mode: bool,

    /// Enable AI player mode
    #[clap(long)]
    ai_player: bool,

    /// Start MCP server mode
    #[clap(long)]
    mcp_server: bool,

    /// Log level (error, warn, info, debug, trace)
    #[clap(long, default_value = "info")]
    log_level: String,
}

#[macroquad::main("Thatch Roguelike")]
async fn main() -> ThatchResult<()> {
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
        return run_ai_player_mode(&args).await;
    }

    // Normal game mode
    info!("Starting in normal game mode");
    run_game(&args).await
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

/// Runs the main game loop with macroquad graphics.
async fn run_game(args: &Args) -> ThatchResult<()> {
    info!("Initializing macroquad display");

    // Configure window for both desktop and mobile
    // On mobile, this will be overridden by the platform
    request_new_screen_size(1024.0, 768.0);

    // Enable high DPI support for mobile
    set_pc_assets_folder("assets");

    // Initialize input handler
    let input_handler = thatch::InputHandler::new();

    run_game_loop(args, &input_handler).await
}

/// Main game loop implementation.
async fn run_game_loop(args: &Args, input_handler: &thatch::InputHandler) -> ThatchResult<()> {
    // Generate a proper dungeon level
    let seed = args.seed.unwrap_or(12345);

    info!("Generating complete 3D dungeon with seed: {}", seed);

    // Initialize game state with complete 3D dungeon (all 26 floors)
    info!("Initializing game state with 3D dungeon generation");
    let mut game_state = GameState::new_with_complete_dungeon(seed)?;

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

    // Initialize scene manager with game state and input handler
    let mut scene_manager = SceneManager::new(game_state, input_handler.clone()).await?;

    // Run the main scene loop
    scene_manager.run().await?;

    info!("Game loop ended");
    Ok(())
}

/// Runs AI player mode for testing and demonstration.
async fn run_ai_player_mode(_args: &Args) -> ThatchResult<()> {
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
