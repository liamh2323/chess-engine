use std::io::{self, BufRead};
use std::time::Instant;

use crate::board::{Game, read_FEN};
use crate::search::{SearchLimits, Searcher};
use crate::types::*;

pub struct UciHandler {
    game: Game,
}

impl Default for UciHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl UciHandler {
    pub fn new() -> Self {
        UciHandler {
            game: Game::initialise(),
        }
    }

    pub fn run(&mut self) {
        let stdin = io::stdin();
        let mut lines = stdin.lock().lines();

        while let Some(Ok(line)) = lines.next() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if line == "quit" || line == "exit" {
                break;
            }

            self.handle_command(line);
        }
    }

    pub fn handle_command(&mut self, line: &str) {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.is_empty() {
            return;
        }

        match tokens[0] {
            "uci" => {
                println!("id name ChessEngine");
                println!("id author Liam Hayes");
                println!("uciok");
            }
            "isready" => {
                println!("readyok");
            }
            "ucinewgame" => {
                self.game = Game::initialise();
            }
            "position" => {
                self.handle_position(&tokens[1..]);
            }
            "go" => {
                self.handle_go(&tokens[1..]);
            }
            "perft" => {
                let depth = tokens.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
                self.run_perft(depth);
            }
            "d" | "print" | "board" => {
                print!("{}", self.game);
                println!("FEN: {}", self.game.to_fen());
            }
            "eval" => {
                let score = self.game.evaluate();
                println!(
                    "Static eval: {} cp (from {:?}'s perspective)",
                    score, self.game.active_colour
                );
            }
            "fen" => {
                println!("{}", self.game.to_fen());
            }
            "moves" => {
                let legal = self.game.generate_legal_moves();
                print!("Legal moves ({}): ", legal.len());
                for m in legal {
                    print!("{} ", m.to_uci());
                }
                println!();
            }
            // If the user types a move directly (e.g. "e2e4")
            candidate_move => {
                if let Some(mv) = self.parse_move_str(candidate_move) {
                    let _undo = self.game.make_move(&mv);
                    print!("{}", self.game);

                    // Check if game is over
                    let responses = self.game.generate_legal_moves();
                    if responses.is_empty() {
                        if self.game.is_in_check(self.game.active_colour) {
                            println!("Checkmate! Game over.");
                        } else {
                            println!("Stalemate! Game drawn.");
                        }
                    } else {
                        // Engine plays response in interactive mode
                        println!("Engine thinking...");
                        let limits = SearchLimits {
                            max_depth: 8,
                            time_limit_ms: Some(1500),
                        };
                        let mut searcher = Searcher::new(&mut self.game);
                        let (best_move, _score) = searcher.search(&limits);
                        if let Some(bm) = best_move {
                            let _undo = self.game.make_move(&bm);
                            println!("Engine plays: {}", bm.to_uci());
                            print!("{}", self.game);
                        }
                    }
                } else {
                    println!("Unknown command: {}", line);
                }
            }
        }
    }

    fn handle_position(&mut self, tokens: &[&str]) {
        if tokens.is_empty() {
            return;
        }

        let mut move_idx = None;

        if tokens[0] == "startpos" {
            self.game = Game::initialise();
            if tokens.len() > 1 && tokens[1] == "moves" {
                move_idx = Some(2);
            }
        } else if tokens[0] == "fen" {
            // Find where "moves" starts, if present
            let mut fen_parts = Vec::new();
            let mut i = 1;
            while i < tokens.len() && tokens[i] != "moves" {
                fen_parts.push(tokens[i]);
                i += 1;
            }
            let fen_str = fen_parts.join(" ");
            self.game = read_FEN(&fen_str);

            if i < tokens.len() && tokens[i] == "moves" {
                move_idx = Some(i + 1);
            }
        }

        if let Some(start) = move_idx {
            for &mv_str in &tokens[start..] {
                if let Some(mv) = self.parse_move_str(mv_str) {
                    self.game.make_move(&mv);
                } else {
                    eprintln!("Invalid move in position command: {}", mv_str);
                    break;
                }
            }
        }
    }

    fn handle_go(&mut self, tokens: &[&str]) {
        let mut limits = SearchLimits {
            max_depth: 64,
            time_limit_ms: None,
        };

        let mut wtime: Option<u64> = None;
        let mut btime: Option<u64> = None;
        let mut winc: u64 = 0;
        let mut binc: u64 = 0;
        let mut movetime: Option<u64> = None;
        let mut depth_limit: Option<usize> = None;

        let mut i = 0;
        while i < tokens.len() {
            match tokens[i] {
                "wtime" => {
                    if i + 1 < tokens.len() {
                        wtime = tokens[i + 1].parse().ok();
                        i += 1;
                    }
                }
                "btime" => {
                    if i + 1 < tokens.len() {
                        btime = tokens[i + 1].parse().ok();
                        i += 1;
                    }
                }
                "winc" => {
                    if i + 1 < tokens.len() {
                        winc = tokens[i + 1].parse().unwrap_or(0);
                        i += 1;
                    }
                }
                "binc" => {
                    if i + 1 < tokens.len() {
                        binc = tokens[i + 1].parse().unwrap_or(0);
                        i += 1;
                    }
                }
                "movetime" => {
                    if i + 1 < tokens.len() {
                        movetime = tokens[i + 1].parse().ok();
                        i += 1;
                    }
                }
                "depth" if i + 1 < tokens.len() => {
                    depth_limit = tokens[i + 1].parse().ok();
                    i += 1;
                }
                _ => {}
            }
            i += 1;
        }

        if let Some(mt) = movetime {
            limits.time_limit_ms = Some(mt);
        } else {
            let (my_time, my_inc) = if self.game.active_colour == Colour::White {
                (wtime, winc)
            } else {
                (btime, binc)
            };

            if let Some(t) = my_time {
                // Standard time budget: remaining_time / 30 + inc / 2
                let budget = (t / 30 + my_inc / 2).max(10u64).min(t.saturating_sub(50));
                limits.time_limit_ms = Some(budget);
            } else if depth_limit.is_none() {
                // Default search time for interactive use
                limits.time_limit_ms = Some(1500);
            }
        }

        if let Some(d) = depth_limit {
            limits.max_depth = d;
        }

        let mut searcher = Searcher::new(&mut self.game);
        let (best_move, _) = searcher.search(&limits);

        if let Some(mv) = best_move {
            println!("bestmove {}", mv.to_uci());
        } else {
            println!("bestmove 0000");
        }
    }

    pub fn parse_move_str(&mut self, s: &str) -> Option<Move> {
        let legal_moves = self.game.generate_legal_moves();
        legal_moves
            .into_iter()
            .find(|mv| mv.to_uci().eq_ignore_ascii_case(s))
    }

    pub fn run_perft(&mut self, depth: usize) {
        let start = Instant::now();
        let divide = self.game.perft_divide(depth);
        let mut total_nodes = 0u64;

        for (mv, count) in divide {
            println!("{}: {}", mv.to_uci(), count);
            total_nodes += count;
        }

        let elapsed = start.elapsed();
        let elapsed_sec = elapsed.as_secs_f64();
        let nps = if elapsed_sec > 0.0 {
            (total_nodes as f64 / elapsed_sec) as u64
        } else {
            0
        };

        println!();
        println!("Nodes searched: {}", total_nodes);
        println!("Time: {:.3} s ({} nps)", elapsed_sec, nps);
    }
}
