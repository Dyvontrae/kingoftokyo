use rand::Rng;
use std::collections::HashMap;
use std::io::{self, Write};

// --- 1. Core Data Structures ---

/// Represents the six possible outcomes of a single die roll.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
enum DieResult {
    One,
    Two,
    Three,
    Energy,     // In-game currency
    Claw,       // Attack/Tokyo
    Heart,      // +1 HP
}

/// Represents a single Kaiju player's state.
#[derive(Debug)]
struct Player {
    id: u32,
    name: String,
    hp: u8,          // Max 12, start 10
    victory_points: u8, // Max 20
    energy: u8,      // Currency
}

impl Player {
    fn new(id: u32, name: &str) -> Self {
        Player {
            id,
            name: name.to_string(),
            hp: 10, // Start HP
            victory_points: 0,
            energy: 0,
        }
    }
}

/// The central Game manager.
struct Game {
    players: Vec<Player>,
    tokyo_controller_id: Option<u32>, // ID of the player currently in Tokyo (or None)
    max_hp: u8,
    max_vp: u8,
}

// --- Helper Function for Reading Input ---

fn read_line_input(prompt: &str) -> String {
    print!("{}", prompt);
    // Flush the output buffer to ensure the prompt is displayed before input
    io::stdout().flush().expect("Failed to flush stdout"); 
    let mut input = String::new();
    // Use read_line to capture the input
    io::stdin().read_line(&mut input).expect("Failed to read line");
    input.trim().to_string()
}

// --- 2. Dice Roll Implementation ---

fn roll_dice() -> [DieResult; 6] {
    let mut rng = rand::thread_rng();
    let mut results = [DieResult::One; 6];

    for i in 0..6 {
        let roll = rng.gen_range(1..=6);
        results[i] = match roll {
            1 => DieResult::One,
            2 => DieResult::Two,
            3 => DieResult::Three,
            4 => DieResult::Energy,
            5 => DieResult::Claw,
            6 => DieResult::Heart,
            _ => unreachable!(),
        };
    }
    results
}

// --- 3. Game Logic Implementation ---

impl Game {
    fn new(player_names: &[&str]) -> Self {
        let players: Vec<Player> = player_names.iter()
            .enumerate()
            .map(|(i, &name)| Player::new(i as u32 + 1, name))
            .collect();

        Game {
            players,
            tokyo_controller_id: None,
            max_hp: 12,
            max_vp: 20,
        }
    }

    /// Finds a player by ID (used for getting mutable access).
    fn get_player_mut(&mut self, player_id: u32) -> Option<&mut Player> {
        self.players.iter_mut().find(|p| p.id == player_id)
    }
    
    /// Finds a player by ID (used for getting read-only access).
    fn get_player(&self, player_id: u32) -> Option<&Player> {
        self.players.iter().find(|p| p.id == player_id)
    }

    /// Awards 2 VP for maintaining Tokyo control at the start of the turn.
    fn apply_tokyo_control_points(&mut self) {
        let max_vp = self.max_vp;

        if let Some(controller_id) = self.tokyo_controller_id {
            if let Some(player) = self.get_player_mut(controller_id) {
                player.victory_points = player.victory_points.saturating_add(2).min(max_vp);
                println!("    ⭐ **{}** MAINTAINS Tokyo control and gains +2 VP! (VP: {})", 
                         player.name, player.victory_points);
            }
        }
    }

    /// Processes all dice results for a player's turn, including user input for decisions.
    fn process_roll(&mut self, player_id: u32, results: &[DieResult; 6]) {
        let max_hp = self.max_hp;
        let max_vp = self.max_vp;

        let mut matched_numbers = 0;
        let player_is_in_tokyo = self.tokyo_controller_id == Some(player_id);

        println!("    Roll Results: {:?}", results);

        // Tally results
        let mut counts: HashMap<DieResult, i32> = HashMap::new(); 
        for &result in results {
            *counts.entry(result).or_insert(0) += 1;
        }

        // --- 1. Scoring: Matched Numbers (3 of a kind) ---
        if counts.get(&DieResult::One).copied().unwrap_or(0) >= 3 { matched_numbers += 1; }
        if counts.get(&DieResult::Two).copied().unwrap_or(0) >= 3 { matched_numbers += 2; }
        if counts.get(&DieResult::Three).copied().unwrap_or(0) >= 3 { matched_numbers += 3; }

        if matched_numbers > 0 {
            if let Some(player) = self.get_player_mut(player_id) {
                player.victory_points = player.victory_points.saturating_add(matched_numbers as u8).min(max_vp);
                println!("    ⭐ Matched numbers gain **{}** VP. (Total VP: {})", matched_numbers, player.victory_points);
            }
        }

        // --- 2. Energy, Hearts, and Claws ---
        let energy_count = counts.get(&DieResult::Energy).copied().unwrap_or(0);
        if energy_count > 0 {
            if let Some(player) = self.get_player_mut(player_id) {
                player.energy = player.energy.saturating_add(energy_count as u8);
                println!("    ⚡ Gains +{} Energy. (Total Energy: {})", energy_count, player.energy);
            }
        }

        let heart_count = counts.get(&DieResult::Heart).copied().unwrap_or(0);
        if heart_count > 0 {
            if !player_is_in_tokyo {
                if let Some(player) = self.get_player_mut(player_id) {
                    player.hp = player.hp.saturating_add(heart_count as u8).min(max_hp); 
                    println!("    ❤️ Gains +{} HP (Outside Tokyo). (Total HP: {})", heart_count, player.hp);
                }
            } else {
                 println!("    ❤️ Heart roll ignored: Player is in Tokyo.");
            }
        }
        
        let claw_count = counts.get(&DieResult::Claw).copied().unwrap_or(0);

        // --- 3. Attack and Tokyo Control ---
        if claw_count > 0 {
            if player_is_in_tokyo {
                // ATTACK: Damage to all OUTSIDE players
                println!("    💥 **ATTACK!** {} deals {} damage from Tokyo.", 
                         self.get_player(player_id).expect("Controller must exist").name, claw_count);

                for other_player in self.players.iter_mut().filter(|p| p.id != player_id) {
                    if self.tokyo_controller_id != Some(other_player.id) {
                         let damage = claw_count as u8;
                         other_player.hp = other_player.hp.saturating_sub(damage);
                         println!("        -> {} takes {} damage! (HP: {})", other_player.name, damage, other_player.hp);
                    }
                }
                
                // DECISION: Concede Tokyo after attacking
                let controller_name = self.get_player(player_id).expect("Player must exist").name.clone();
                let input = read_line_input(&format!("\n    ❓ {} has finished attacking. CONCEDE Tokyo? (y/N): ", controller_name));
                
                if input.eq_ignore_ascii_case("y") {
                    println!("    📢 {} CONCEDES Tokyo!", controller_name);
                    self.tokyo_controller_id = None;
                }

            } else {
                // CONTEST/ENTER TOKYO
                let current_controller = self.tokyo_controller_id;
                let player_name = self.get_player(player_id).expect("Player must exist").name.clone();

                let mut should_enter = false;

                if let Some(id) = current_controller {
                    // Tokyo is occupied. Challenger rolls claws.
                    let controller_name = self.get_player(id).expect("Controller must exist").name.clone();
                    
                    let input = read_line_input(&format!("\n    ⚔️  {} challenges {} with {} Claw(s). Should {} CONCEDE Tokyo? (y/N): ", 
                                                         player_name, controller_name, claw_count, controller_name));
                    
                    if input.eq_ignore_ascii_case("y") {
                        println!("    📢 {} CONCEDES Tokyo!", controller_name);
                        self.tokyo_controller_id = None; // Tokyo is now vacant
                        should_enter = true;
                    } else {
                        println!("    🛡️ {} holds Tokyo against {}'s challenge.", controller_name, player_name);
                        return; // No change in control
                    }
                } else {
                    // Tokyo is vacant
                    should_enter = true;
                }

                if should_enter {
                    let input = read_line_input(&format!("    ❓ Tokyo is vacant. {} rolled {} Claw(s). Do you want to ENTER Tokyo? (Y/n): ", player_name, claw_count));

                    if !input.eq_ignore_ascii_case("n") {
                        self.tokyo_controller_id = Some(player_id);
                        if let Some(player) = self.get_player_mut(player_id) {
                            player.victory_points = player.victory_points.saturating_add(1).min(max_vp);
                            println!("    🚪 **{}** ENTERS Tokyo and gains +1 VP! (Total VP: {})", 
                                    player.name, player.victory_points);
                        }
                    } else {
                         println!("    🚫 {} declines to enter Tokyo.", player_name);
                    }
                }
            }
        }
    }

    /// Checks if the game has ended based on VP or HP conditions.
    fn check_victory_condition(&self) -> Option<String> {
        let active_players: Vec<&Player> = self.players.iter().filter(|p| p.hp > 0).collect();
        let max_vp = self.max_vp;

        // VP WIN
        if let Some(winner) = active_players.iter().find(|p| p.victory_points >= max_vp) {
            return Some(format!("{} reached {} Victory Points!", winner.name, max_vp));
        }

        // HP WIN (Last Kaiju Standing)
        if active_players.len() <= 1 {
            return if let Some(winner) = active_players.first() {
                Some(format!("{} is the Last Kaiju Standing!", winner.name))
            } else {
                // All players eliminated simultaneously
                Some(String::from("All Kaiju were eliminated simultaneously!"))
            };
        }

        None
    }
} 

// --- 4. Main Game Loop Implementation (Full Interactive Flow) ---

fn main() {
    println!("# 🦖 KING OF TOKYO (Simplified) 🏙️ #");
    
    // -----------------------------------------------------
    // Game Setup
    // -----------------------------------------------------
    let num_players_str = read_line_input("How many players (2-6)? ");
    let num_players: usize = num_players_str.parse().unwrap_or(2).min(6).max(2);
    
    let mut player_names = Vec::new();
    for i in 0..num_players {
        let name = read_line_input(&format!("Enter name for Player {}: ", i + 1));
        player_names.push(name);
    }
    
    let player_refs: Vec<&str> = player_names.iter().map(|s| s.as_str()).collect();
    let mut game = Game::new(&player_refs);
    
    println!("\n--- Game Start with {} Players ---", num_players);
    // -----------------------------------------------------
    
    let mut turn_count = 1;
    let mut current_player_index = 0;

    loop {
        // Ensure index is within bounds and cycles
        current_player_index %= game.players.len(); 

        let player_index = current_player_index;
        let current_player_id = game.players[player_index].id;
        let current_player_name = game.players[player_index].name.clone();
        
        // Skip dead players
        if game.players[player_index].hp == 0 {
            current_player_index += 1;
            continue;
        }

        println!("\n---------------------------------------------------------");
        println!("--- Turn {} - {}'s Turn (HP: {}, VP: {}) ---", 
                 turn_count, 
                 current_player_name, 
                 game.players[player_index].hp,
                 game.players[player_index].victory_points);
        println!("---------------------------------------------------------");
        
        // 1. Check for passive Tokyo VP
        game.apply_tokyo_control_points();

        // 2. Check for victory after Tokyo VP
        if let Some(message) = game.check_victory_condition() {
            println!("\n### 🎉 GAME OVER! ###");
            println!("{}", message);
            break;
        }

        // 3. Roll Dice
        let dice_results = roll_dice();
        
        // 4. Process Roll (Handles scoring, attack, and interactive Tokyo decisions)
        game.process_roll(current_player_id, &dice_results);

        // 5. Check for victory after roll effects
        if let Some(message) = game.check_victory_condition() {
            println!("\n### 🎉 GAME OVER! ###");
            println!("{}", message);
            break;
        }

        // Move to next player
        current_player_index += 1;
        turn_count += 1;

        if turn_count > 1000 { 
            println!("\nGame stopped after 1000 turns for simulation limit.");
            break;
        }
    }
    
    // --- Final Tally ---
    println!("\n--- Final Scores ---");
    for player in game.players {
        println!("- {}: {} VP, {} HP, {} Energy", player.name, player.victory_points, player.hp, player.energy);
    }
}