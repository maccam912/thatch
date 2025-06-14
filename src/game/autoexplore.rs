//! # Autoexplore Module
//!
//! Debug functionality for automatically exploring dungeons and navigating between levels.

use crate::{
    GameState, Position, ThatchError, ThatchResult, TileType, Direction,
    ConcreteAction, MoveAction, UseStairsAction, StairDirection, Entity,
};
use std::collections::{HashMap, BinaryHeap};
use std::cmp::Ordering;

/// Autoexplore state and functionality for debug mode.
#[derive(Debug, Clone)]
pub struct AutoexploreState {
    /// Whether autoexplore is currently enabled
    pub enabled: bool,
    /// Current path being followed
    pub current_path: Vec<Position>,
    /// Current target position
    pub target: Option<Position>,
    /// Last action execution time for speed control
    pub last_action_time: Option<std::time::Instant>,
    /// Delay between actions in milliseconds
    pub action_delay_ms: u64,
}

impl AutoexploreState {
    /// Creates a new autoexplore state.
    pub fn new() -> Self {
        Self {
            enabled: false,
            current_path: Vec::new(),
            target: None,
            last_action_time: None,
            action_delay_ms: 200, // 200ms between actions = 5 actions per second
        }
    }

    /// Toggles autoexplore on/off.
    pub fn toggle(&mut self) -> bool {
        self.enabled = !self.enabled;
        if !self.enabled {
            // Clear state when disabling
            self.current_path.clear();
            self.target = None;
            self.last_action_time = None;
        }
        self.enabled
    }

    /// Checks if enough time has passed for the next action.
    pub fn can_perform_action(&self) -> bool {
        if let Some(last_time) = self.last_action_time {
            last_time.elapsed().as_millis() >= self.action_delay_ms as u128
        } else {
            true // First action
        }
    }

    /// Updates the last action time.
    pub fn mark_action_performed(&mut self) {
        self.last_action_time = Some(std::time::Instant::now());
    }

    /// Gets the next autoexplore action to perform.
    pub fn get_next_action(&mut self, game_state: &GameState) -> ThatchResult<Option<ConcreteAction>> {
        if !self.enabled || !self.can_perform_action() {
            return Ok(None);
        }

        let player = game_state.get_player()
            .ok_or_else(|| ThatchError::InvalidState("No player found".to_string()))?;
        let player_pos = player.position();
        let player_id = player.id();

        // Check if we're already on stairs down
        if let Some(level) = game_state.world.current_level() {
            if let Some(tile) = level.get_tile(player_pos) {
                if tile.tile_type == TileType::StairsDown {
                    // We're on stairs down, use them
                    self.mark_action_performed();
                    return Ok(Some(ConcreteAction::UseStairs(UseStairsAction::new(
                        player_id,
                        StairDirection::Down,
                    ))));
                }
            }
        }

        // If we have a current path, follow it
        if !self.current_path.is_empty() {
            let next_pos = self.current_path.remove(0);
            if let Some(direction) = self.get_direction_to_position(player_pos, next_pos) {
                self.mark_action_performed();
                return Ok(Some(ConcreteAction::Move(MoveAction {
                    actor: player_id,
                    direction,
                    metadata: HashMap::new(),
                })));
            } else {
                // Path is invalid, clear it
                self.current_path.clear();
            }
        }

        // We need a new path - find stairs down
        if let Some(stairs_down_pos) = self.find_stairs_down(game_state) {
            if let Some(path) = self.find_path(game_state, player_pos, stairs_down_pos)? {
                self.current_path = path;
                self.target = Some(stairs_down_pos);
                
                // Return the first move in the path
                if !self.current_path.is_empty() {
                    let next_pos = self.current_path.remove(0);
                    if let Some(direction) = self.get_direction_to_position(player_pos, next_pos) {
                        self.mark_action_performed();
                        return Ok(Some(ConcreteAction::Move(MoveAction {
                            actor: player_id,
                            direction,
                            metadata: HashMap::new(),
                        })));
                    }
                }
            }
        }

        // No stairs down found or no path available
        Ok(None)
    }

    /// Finds the position of stairs down on the current level.
    fn find_stairs_down(&self, game_state: &GameState) -> Option<Position> {
        let level = game_state.world.current_level()?;
        level.stairs_down_position
    }

    /// Gets the direction from one position to an adjacent position.
    fn get_direction_to_position(&self, from: Position, to: Position) -> Option<Direction> {
        let delta = to - from;
        Direction::from_delta(delta)
    }

    /// Uses A* pathfinding to find a path between two positions.
    pub fn find_path(
        &self,
        game_state: &GameState,
        start: Position,
        goal: Position,
    ) -> ThatchResult<Option<Vec<Position>>> {
        let level = game_state.world.current_level()
            .ok_or_else(|| ThatchError::InvalidState("No current level".to_string()))?;

        // A* algorithm implementation
        let mut open_set = BinaryHeap::new();
        let mut came_from = HashMap::new();
        let mut g_score = HashMap::new();
        let mut f_score = HashMap::new();

        g_score.insert(start, 0.0);
        f_score.insert(start, start.euclidean_distance(goal));
        open_set.push(AStarNode {
            position: start,
            f_score: start.euclidean_distance(goal),
        });

        while let Some(current_node) = open_set.pop() {
            let current = current_node.position;

            if current == goal {
                // Reconstruct path
                let mut path = Vec::new();
                let mut current_pos = goal;
                
                while let Some(&prev) = came_from.get(&current_pos) {
                    path.push(current_pos);
                    current_pos = prev;
                }
                
                path.reverse();
                return Ok(Some(path));
            }

            // Check all adjacent positions
            for neighbor in current.adjacent_positions() {
                if !level.is_valid_position(neighbor) {
                    continue;
                }

                // Check if tile is passable
                let tile = level.get_tile(neighbor).unwrap();
                if !tile.tile_type.is_passable() {
                    continue;
                }

                // Check if there's an entity blocking the path (except at goal)
                if neighbor != goal && game_state.get_entity_at_position(neighbor).is_some() {
                    continue;
                }

                let tentative_g_score = g_score.get(&current).unwrap_or(&f64::INFINITY) + 1.0;

                if tentative_g_score < *g_score.get(&neighbor).unwrap_or(&f64::INFINITY) {
                    came_from.insert(neighbor, current);
                    g_score.insert(neighbor, tentative_g_score);
                    let f = tentative_g_score + neighbor.euclidean_distance(goal);
                    f_score.insert(neighbor, f);

                    // Add to open set if not already there with a better score
                    open_set.push(AStarNode {
                        position: neighbor,
                        f_score: f,
                    });
                }
            }
        }

        Ok(None) // No path found
    }
}

impl Default for AutoexploreState {
    fn default() -> Self {
        Self::new()
    }
}

/// Node for A* pathfinding algorithm.
#[derive(Debug, Clone)]
pub struct AStarNode {
    pub position: Position,
    pub f_score: f64,
}

impl PartialEq for AStarNode {
    fn eq(&self, other: &Self) -> bool {
        self.f_score == other.f_score
    }
}

impl Eq for AStarNode {}

impl PartialOrd for AStarNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AStarNode {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering for min-heap behavior in BinaryHeap
        other
            .f_score
            .partial_cmp(&self.f_score)
            .unwrap_or(Ordering::Equal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Level, Tile, PlayerCharacter, ConcreteEntity, GameState};

    #[test]
    fn test_autoexplore_state_creation() {
        let autoexplore = AutoexploreState::new();
        assert!(!autoexplore.enabled);
        assert!(autoexplore.current_path.is_empty());
        assert!(autoexplore.target.is_none());
    }

    #[test]
    fn test_autoexplore_toggle() {
        let mut autoexplore = AutoexploreState::new();
        
        // Toggle on
        assert!(autoexplore.toggle());
        assert!(autoexplore.enabled);
        
        // Toggle off
        assert!(!autoexplore.toggle());
        assert!(!autoexplore.enabled);
    }

    #[test]
    fn test_direction_calculation() {
        let autoexplore = AutoexploreState::new();
        
        let from = Position::new(5, 5);
        let to = Position::new(5, 4); // North
        assert_eq!(autoexplore.get_direction_to_position(from, to), Some(Direction::North));
        
        let to = Position::new(6, 5); // East
        assert_eq!(autoexplore.get_direction_to_position(from, to), Some(Direction::East));
        
        let to = Position::new(4, 6); // Southwest
        assert_eq!(autoexplore.get_direction_to_position(from, to), Some(Direction::Southwest));
    }

    #[test]
    fn test_pathfinding() {
        let autoexplore = AutoexploreState::new();
        
        // Create a simple level
        let mut level = Level::new(0, 10, 10);
        
        // Create a corridor from (1,1) to (8,1)
        for x in 1..9 {
            level.set_tile(Position::new(x, 1), Tile::floor()).unwrap();
        }
        
        // Create game state
        let mut game_state = GameState::new_with_level(level, 12345).unwrap();
        
        // Test pathfinding
        let start = Position::new(1, 1);
        let goal = Position::new(8, 1);
        
        let path = autoexplore.find_path(&game_state, start, goal).unwrap();
        assert!(path.is_some());
        
        let path = path.unwrap();
        assert!(!path.is_empty());
        assert_eq!(path[path.len() - 1], goal);
    }
}