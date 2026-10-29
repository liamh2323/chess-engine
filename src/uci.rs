use std::io::{self, BufRead};
use std::time::Instant;

use crate::board::{Game, UndoInfo};
use crate::search::{SearchLimits, Searcher};
use crate::tt::TranspositionTable;
use crate::types::*;

const BENCH_FENS: [&str; 6] = [
    "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
    "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
    "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
    "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
    "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
    "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
];

pub struct UciHandler {
    pub game: Game,
    pub tt: TranspositionTable,
    pub move_history: Vec<(Move, UndoInfo)>,
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
            tt: TranspositionTable::new_mb(16),
            move_history: Vec::new(),
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
                println!("option name Hash type spin default 16 min 1 max 1024");
                println!("option name Clear Hash type button");
                println!("uciok");
            }
            "isready" => {
                println!("readyok");
            }
            "setoption" => {
                self.handle_setoption(&tokens[1..]);
            }
            "ucinewgame" => {
                self.game = Game::initialise();
                self.tt.clear();
                self.move_history.clear();
            }
            "position" => {
                self.handle_position(&tokens[1..]);
            }
            "go" => {
                self.handle_go(&tokens[1..]);
            }
            "stop" => {}
            "perft" => {
                let depth = tokens.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
                self.run_perft(depth);
            }
            "bench" => {
                let depth = tokens.get(1).and_then(|s| s.parse().ok()).unwrap_or(6);
                self.run_bench(depth);
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
            "undo" => {
                if let Some((mv, undo)) = self.move_history.pop() {
                    self.game.unmake_move(&mv, &undo);
                    print!("{}", self.game);
                } else {
                    println!("No moves to undo.");
                }
            }
            "help" => {
                println!("Available commands:");
                println!("  uci                        - Initialize UCI mode");
                println!("  isready                    - Query engine readiness");
                println!("  ucinewgame                 - Reset board and clear hash table");
                println!("  setoption name <N> [value <V>] - Configure UCI option (Hash, Clear Hash)");
                println!("  position [startpos|fen <FEN>] [moves <m1> <m2> ...]");
                println!("  go [depth <D>] [movetime <MS>] [nodes <N>] [mate <M>] [wtime <MS>] [btime <MS>] [perft <D>]");
                println!("  perft <D>                  - Run perft divide to depth D");
                println!("  bench [D]                  - Run benchmark across standard positions to depth D");
                println!("  d / board                  - Display board and FEN");
                println!("  eval                       - Display static evaluation");
                println!("  moves                      - List all legal moves");
                println!("  undo                       - Undo last move");
                println!("  <move> (e.g. e2e4 or Nf3)  - Play a move interactively against the engine");
                println!("  quit / exit                - Exit the program");
            }
            // If the user types a move directly (e.g. "e2e4" or "Nf3")
            candidate_move => {
                if let Some(mv) = self.parse_move_str(candidate_move) {
                    let undo = self.game.make_move(&mv);
                    self.move_history.push((mv, undo));
                    print!("{}", self.game);

                    if self.check_and_report_game_over() {
                        return;
                    }

                    // Engine plays response in interactive mode
                    println!("Engine thinking...");
                    let limits = SearchLimits {
                        max_depth: 8,
                        time_limit_ms: Some(1500),
                    };
                    let mut searcher = Searcher::with_tt(&mut self.game, &mut self.tt);
                    let (best_move, _score) = searcher.search(&limits);
                    if let Some(bm) = best_move {
                        let san = self.game.move_to_san(&bm);
                        let undo = self.game.make_move(&bm);
                        self.move_history.push((bm, undo));
                        println!("Engine plays: {} ({})", bm.to_uci(), san);
                        print!("{}", self.game);
                        let _ = self.check_and_report_game_over();
                    }
                } else {
                    println!("Unknown command: {}", line);
                }
            }
        }
    }

    fn check_and_report_game_over(&mut self) -> bool {
        let responses = self.game.generate_legal_moves();
        if responses.is_empty() {
            if self.game.is_in_check(self.game.active_colour) {
                println!("Checkmate! Game over.");
            } else {
                println!("Stalemate! Game drawn.");
            }
            return true;
        }
        if self.game.is_threefold_repetition() {
            println!("Draw by threefold repetition!");
            return true;
        }
        if self.game.is_fifty_move_draw() {
            println!("Draw by fifty-move rule!");
            return true;
        }
        if self.game.is_insufficient_material() {
            println!("Draw by insufficient material!");
            return true;
        }
        false
    }

    fn handle_setoption(&mut self, tokens: &[&str]) {
        // Format: name <option_name> [value <option_value>]
        let mut name_parts = Vec::new();
        let mut value_parts = Vec::new();
        let mut in_name = false;
        let mut in_value = false;

        for &tok in tokens {
            if tok.eq_ignore_ascii_case("name") {
                in_name = true;
                in_value = false;
            } else if tok.eq_ignore_ascii_case("value") {
                in_name = false;
                in_value = true;
            } else if in_name {
                name_parts.push(tok);
            } else if in_value {
                value_parts.push(tok);
            }
        }

        let name = name_parts.join(" ").to_ascii_lowercase();
        let value = value_parts.join(" ");

        match name.as_str() {
            "hash" => {
                if let Ok(mb) = value.parse::<usize>() {
                    self.tt = TranspositionTable::new_mb(mb);
                }
            }
            "clear hash" => {
                self.tt.clear();
            }
            _ => {}
        }
    }

    fn handle_position(&mut self, tokens: &[&str]) {
        if tokens.is_empty() {
            return;
        }

        let mut move_idx = None;

        if tokens[0] == "startpos" {
            self.game = Game::initialise();
            self.move_history.clear();
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
            match Game::from_fen(&fen_str) {
                Ok(game) => {
                    self.game = game;
                    self.move_history.clear();
                }
                Err(e) => {
                    eprintln!("Invalid FEN '{}': {}", fen_str, e);
                    return;
                }
            }

            if i < tokens.len() && tokens[i] == "moves" {
                move_idx = Some(i + 1);
            }
        }

        if let Some(start) = move_idx {
            for &mv_str in &tokens[start..] {
                if let Some(mv) = self.parse_move_str(mv_str) {
                    let undo = self.game.make_move(&mv);
                    self.move_history.push((mv, undo));
                } else {
                    eprintln!("Invalid move in position command: {}", mv_str);
                    break;
                }
            }
        }
    }

    fn handle_go(&mut self, tokens: &[&str]) {
        if !tokens.is_empty() && tokens[0] == "perft" {
            let depth = tokens.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
            self.run_perft(depth);
            return;
        }

        let mut limits = SearchLimits {
            max_depth: 64,
            time_limit_ms: None,
        };

        let mut wtime: Option<u64> = None;
        let mut btime: Option<u64> = None;
        let mut winc: u64 = 0;
        let mut binc: u64 = 0;
        let mut movestogo: u64 = 30;
        let mut movetime: Option<u64> = None;
        let mut depth_limit: Option<usize> = None;
        let mut node_limit: Option<u64> = None;
        let mut mate_limit: Option<usize> = None;
        let mut soft_time_limit: Option<u64> = None;
        let mut infinite = false;

        let mut i = 0;
        while i < tokens.len() {
            match tokens[i] {
                "wtime" if i + 1 < tokens.len() => {
                    wtime = tokens[i + 1].parse().ok();
                    i += 1;
                }
                "btime" if i + 1 < tokens.len() => {
                    btime = tokens[i + 1].parse().ok();
                    i += 1;
                }
                "winc" if i + 1 < tokens.len() => {
                    winc = tokens[i + 1].parse().unwrap_or(0);
                    i += 1;
                }
                "binc" if i + 1 < tokens.len() => {
                    binc = tokens[i + 1].parse().unwrap_or(0);
                    i += 1;
                }
                "movestogo" if i + 1 < tokens.len() => {
                    movestogo = tokens[i + 1].parse().unwrap_or(30).max(1);
                    i += 1;
                }
                "movetime" if i + 1 < tokens.len() => {
                    movetime = tokens[i + 1].parse().ok();
                    i += 1;
                }
                "depth" if i + 1 < tokens.len() => {
                    depth_limit = tokens[i + 1].parse().ok();
                    i += 1;
                }
                "nodes" if i + 1 < tokens.len() => {
                    node_limit = tokens[i + 1].parse().ok();
                    i += 1;
                }
                "mate" if i + 1 < tokens.len() => {
                    mate_limit = tokens[i + 1].parse().ok();
                    i += 1;
                }
                "infinite" => {
                    infinite = true;
                }
                _ => {}
            }
            i += 1;
        }

        if infinite {
            limits.time_limit_ms = None;
        } else if let Some(mt) = movetime {
            limits.time_limit_ms = Some(mt);
        } else {
            let (my_time, my_inc) = if self.game.active_colour == Colour::White {
                (wtime, winc)
            } else {
                (btime, binc)
            };

            if let Some(t) = my_time {
                let overhead = 30u64;
                let safe_max = t.saturating_sub(overhead).max(5);
                let target = (t / movestogo + (my_inc * 3) / 4)
                    .max(10u64)
                    .min(safe_max);
                let hard_budget = (target * 2).min(safe_max);
                soft_time_limit = Some(target);
                limits.time_limit_ms = Some(hard_budget);
            } else if depth_limit.is_none() && node_limit.is_none() && mate_limit.is_none() {
                // Default search time for interactive use
                limits.time_limit_ms = Some(1500);
            }
        }

        if let Some(d) = depth_limit {
            limits.max_depth = d;
        } else if let Some(m) = mate_limit {
            limits.max_depth = (m * 2 + 1).min(63);
        }

        let mut searcher = Searcher::with_tt(&mut self.game, &mut self.tt);
        searcher.node_limit = node_limit;
        searcher.mate_limit = mate_limit;
        searcher.soft_time_limit_ms = soft_time_limit;
        let (best_move, _) = searcher.search(&limits);

        if let Some(mv) = best_move {
            println!("bestmove {}", mv.to_uci());
        } else {
            println!("bestmove 0000");
        }
    }

    pub fn parse_move_str(&mut self, s: &str) -> Option<Move> {
        self.game.parse_move(s)
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

    pub fn run_bench(&mut self, depth: usize) {
        let start = Instant::now();
        let mut total_nodes = 0u64;
        let saved_game = self.game.clone();
        let saved_history = self.move_history.clone();

        for (idx, fen) in BENCH_FENS.iter().enumerate() {
            println!("\nPosition {}/{}: {}", idx + 1, BENCH_FENS.len(), fen);
            self.tt.clear();
            if let Ok(g) = Game::from_fen(fen) {
                self.game = g;
                let limits = SearchLimits {
                    max_depth: depth,
                    time_limit_ms: None,
                };
                let mut searcher = Searcher::with_tt(&mut self.game, &mut self.tt);
                let (best_mv, score) = searcher.search(&limits);
                total_nodes += searcher.nodes;
                println!(
                    "Result: bestmove {} (score {} cp, {} nodes)",
                    best_mv.map(|m| m.to_uci()).unwrap_or_else(|| "0000".to_string()),
                    score,
                    searcher.nodes
                );
            }
        }

        self.game = saved_game;
        self.move_history = saved_history;

        let elapsed = start.elapsed();
        let elapsed_sec = elapsed.as_secs_f64();
        let nps = if elapsed_sec > 0.0 {
            (total_nodes as f64 / elapsed_sec) as u64
        } else {
            0
        };

        println!("\n========================================");
        println!("Total time : {:.3} s", elapsed_sec);
        println!("Total nodes: {}", total_nodes);
        println!("Overall NPS: {} nodes/sec", nps);
        println!("========================================");
    }
}
