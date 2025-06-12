# Thatch Roguelike Development Instructions

## Project Overview
Thatch is a deep, complex roguelike game written in Rust that integrates an LLM as a Dungeon Master (LLDM). The game features procedural dungeon generation with LLM-generated unique items and narrative elements.

## Architecture Goals
- **MCP Integration**: Design all systems to be eventually controllable via MCP (Model Context Protocol)
- **Modular Design**: Use traits and structs extensively for clean separation of concerns
- **LLDM Integration**: Prepare integration points for LLM-driven content generation and game mastering

## Code Quality Standards

### Documentation
- **Every** public function, struct, enum, and trait MUST have comprehensive rustdoc comments
- Include examples in documentation where appropriate
- Document all panics, errors, and edge cases
- Use `#[doc = "..."]` for complex documentation
- Include module-level documentation explaining the purpose and relationships

### Function and Type Naming
- Use descriptive, self-documenting names: `calculate_line_of_sight` not `calc_los`
- Structs: PascalCase with descriptive names (`DungeonGenerator`, `PlayerCharacter`)
- Functions: snake_case with verb-based names (`generate_dungeon_level`, `move_player_to_position`)
- Enums: PascalCase variants (`TileType::Wall`, `ActionResult::Success`)
- Constants: SCREAMING_SNAKE_CASE (`MAX_DUNGEON_WIDTH`, `DEFAULT_PLAYER_HEALTH`)

### Testing Requirements
- **Unit tests** for every non-trivial function
- **Integration tests** for major systems
- **Property-based tests** using `proptest` for game mechanics
- **Documentation tests** to ensure examples work
- Minimum 90% code coverage
- Test edge cases, error conditions, and boundary values

### Error Handling
- Use `Result<T, E>` for fallible operations
- Create custom error types using `thiserror`
- Never use `unwrap()` or `expect()` in production code paths
- Provide meaningful error messages

### Performance Considerations
- Profile critical paths (rendering, pathfinding, generation)
- Use appropriate data structures for game state
- Consider memory allocation patterns in tight loops

## Project Structure
```
src/
├── main.rs                 # Entry point and game initialization
├── lib.rs                  # Library exports and module declarations
├── game/
│   ├── mod.rs             # Game state management
│   ├── world.rs           # World and level representation
│   ├── entities.rs        # Player, monsters, items
│   └── actions.rs         # Game actions and effects
├── generation/
│   ├── mod.rs             # Procedural generation systems
│   ├── dungeon.rs         # Dungeon layout generation
│   ├── items.rs           # Item generation (future LLDM integration)
│   └── encounters.rs      # Encounter generation
├── rendering/
│   ├── mod.rs             # Terminal rendering system
│   ├── display.rs         # Screen management
│   └── ui.rs              # User interface elements
├── input/
│   ├── mod.rs             # Input handling and command parsing
│   └── commands.rs        # Command definitions
├── lldm/
│   ├── mod.rs             # LLM Dungeon Master integration
│   ├── traits.rs          # Traits for LLM interactions
│   └── mcp.rs             # MCP server integration (future)
└── utils/
    ├── mod.rs             # Utility functions
    ├── math.rs            # Game mathematics
    └── pathfinding.rs     # Pathfinding algorithms
```

## Dependencies to Include
- `crossterm`: Terminal manipulation
- `serde`: Serialization for save/load and MCP
- `tokio`: Async runtime for future LLM integration
- `thiserror`: Error handling
- `proptest`: Property-based testing
- `criterion`: Benchmarking
- `log`: Logging
- `env_logger`: Log output
- `rand`: Random number generation
- `uuid`: Unique identifiers

## MCP Integration Planning
- Design all game actions as discrete, serializable commands
- Use trait objects for extensibility
- Prepare JSON-RPC compatible interfaces
- Consider async operations for LLM calls

## Testing Strategy
- Unit tests in each module file
- Integration tests in `tests/` directory
- Property-based tests for:
  - Dungeon generation (connectivity, bounds)
  - Combat calculations
  - Inventory management
  - Movement validation
- Benchmark critical algorithms

## Immediate Development Priority
1. Core game types and traits
2. Procedural dungeon generation
3. Player movement system
4. Terminal rendering
5. Basic game loop
6. Comprehensive test suite

Remember: Every line of code should be clear, well-documented, and thoroughly tested. This is a complex system that will grow significantly, so invest in quality from the beginning.