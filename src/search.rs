use std::time::Instant;

use crate::board::Game;
use crate::types::*;

pub const INFINITY: i32 = 1_000_000;
pub const MATE_SCORE: i32 = 30_000;

#[derive(Debug, Clone)]
pub struct SearchLimits {
    pub max_depth: usize,
    pub time_limit_ms: Option<u64>,
}

impl Default for SearchLimits {
    fn default() -> Self {
        SearchLimits {
            max_depth: 64,
            time_limit_ms: Some(1000), // Default 1 second
        }
    }
}

pub struct Searcher<'a> {
    pub game: &'a mut Game,
    pub nodes: u64,
    pub start_time: Instant,
    pub time_limit_ms: Option<u64>,
    pub stopped: bool,
    pub killer_moves: [[Option<Move>; 2]; 64],
}

impl<'a> Searcher<'a> {
    pub fn new(game: &'a mut Game) -> Self {
        Searcher {
            game,
            nodes: 0,
            start_time: Instant::now(),
            time_limit_ms: None,
            stopped: false,
            killer_moves: [[None; 2]; 64],
        }
    }

    fn check_time(&mut self) {
        if self.nodes.is_multiple_of(2048)
            && let Some(limit) = self.time_limit_ms
            && self.start_time.elapsed().as_millis() as u64 >= limit
        {
            self.stopped = true;
        }
    }

    pub fn search(&mut self, limits: &SearchLimits) -> (Option<Move>, i32) {
        self.nodes = 0;
        self.start_time = Instant::now();
        self.time_limit_ms = limits.time_limit_ms;
        self.stopped = false;
        self.killer_moves = [[None; 2]; 64];

        let mut best_move = None;
        let mut best_score = 0;

        let legal_moves = self.game.generate_legal_moves();
        if legal_moves.is_empty() {
            return (None, 0);
        }
        if legal_moves.len() == 1 {
            return (Some(legal_moves[0]), 0);
        }

        for depth in 1..=limits.max_depth {
            let mut alpha = -INFINITY;
            let beta = INFINITY;
            let mut current_best_move = None;
            let mut current_best_score = -INFINITY;

            let mut ordered_moves = legal_moves.clone();
            self.order_moves(&mut ordered_moves, 0, best_move);

            for mv in ordered_moves {
                let undo = self.game.make_move(&mv);
                let score = -self.alpha_beta(depth - 1, -beta, -alpha, 1);
                self.game.unmake_move(&mv, &undo);

                if self.stopped {
                    break;
                }

                if score > current_best_score {
                    current_best_score = score;
                    current_best_move = Some(mv);
                }
                if score > alpha {
                    alpha = score;
                }
            }

            if self.stopped && depth > 1 {
                break;
            }

            if let Some(mv) = current_best_move {
                best_move = Some(mv);
                best_score = current_best_score;

                let elapsed_ms = self.start_time.elapsed().as_millis().max(1);
                let nps = (self.nodes as u128 * 1000) / elapsed_ms;

                let score_str = if best_score.abs() > MATE_SCORE - 100 {
                    let mate_plies = MATE_SCORE - best_score.abs();
                    let mate_moves = (mate_plies + 1) / 2;
                    let sign = if best_score > 0 { "" } else { "-" };
                    format!("mate {}{}", sign, mate_moves)
                } else {
                    format!("cp {}", best_score)
                };

                println!(
                    "info depth {} score {} time {} nodes {} nps {} pv {}",
                    depth,
                    score_str,
                    elapsed_ms,
                    self.nodes,
                    nps,
                    mv.to_uci()
                );

                if best_score.abs() > MATE_SCORE - 100 {
                    // Found forced mate, no need to search deeper
                    break;
                }
            }
        }

        (
            best_move.or_else(|| legal_moves.first().copied()),
            best_score,
        )
    }

    fn alpha_beta(&mut self, depth: usize, mut alpha: i32, beta: i32, ply: usize) -> i32 {
        self.nodes += 1;
        self.check_time();
        if self.stopped {
            return 0;
        }

        // Draw by 50-move rule
        if self.game.halfmove_clock >= 100 {
            return 0;
        }

        if depth == 0 {
            return self.quiescence(alpha, beta);
        }

        let in_check = self.game.is_in_check(self.game.active_colour);
        // Check extension: if in check, search one ply deeper
        let depth = if in_check { depth + 1 } else { depth };

        let mut moves = self.game.generate_legal_moves();
        if moves.is_empty() {
            if in_check {
                return -MATE_SCORE + ply as i32;
            } else {
                return 0; // Stalemate
            }
        }

        self.order_moves(&mut moves, ply, None);

        let mut best_score = -INFINITY;

        for mv in moves {
            let undo = self.game.make_move(&mv);
            let score = -self.alpha_beta(depth - 1, -beta, -alpha, ply + 1);
            self.game.unmake_move(&mv, &undo);

            if self.stopped {
                return 0;
            }

            if score > best_score {
                best_score = score;
            }
            if score > alpha {
                alpha = score;
            }
            if alpha >= beta {
                // Killer move heuristic
                if mv.promotion.is_none() && self.game.squares[mv.to] == Square::Empty && ply < 64 {
                    self.killer_moves[ply][1] = self.killer_moves[ply][0];
                    self.killer_moves[ply][0] = Some(mv);
                }
                break; // Beta cutoff
            }
        }

        best_score
    }

    fn quiescence(&mut self, mut alpha: i32, beta: i32) -> i32 {
        self.nodes += 1;
        self.check_time();
        if self.stopped {
            return 0;
        }

        let stand_pat = self.game.evaluate();
        if stand_pat >= beta {
            return beta;
        }
        if stand_pat > alpha {
            alpha = stand_pat;
        }

        let pseudo_moves = self.game.generate_pseudo_legal_moves();
        let us = self.game.active_colour;

        // In quiescence, only examine captures and promotions
        let mut capture_moves: Vec<Move> = pseudo_moves
            .into_iter()
            .filter(|m| {
                m.is_en_passant || m.promotion.is_some() || self.game.squares[m.to] != Square::Empty
            })
            .collect();

        self.order_captures(&mut capture_moves);

        for mv in capture_moves {
            let undo = self.game.make_move(&mv);
            if self.game.is_in_check(us) {
                self.game.unmake_move(&mv, &undo);
                continue;
            }

            let score = -self.quiescence(-beta, -alpha);
            self.game.unmake_move(&mv, &undo);

            if self.stopped {
                return 0;
            }

            if score >= beta {
                return beta;
            }
            if score > alpha {
                alpha = score;
            }
        }

        alpha
    }

    fn score_move(&self, mv: &Move, ply: usize, pv_move: Option<Move>) -> i32 {
        if Some(*mv) == pv_move {
            return 1_000_000;
        }

        let is_capture = self.game.squares[mv.to] != Square::Empty || mv.is_en_passant;
        if is_capture {
            let victim_val = if mv.is_en_passant {
                100
            } else if let Square::Occupied(idx) = self.game.squares[mv.to] {
                self.piece_value(self.game.pieces[idx].piece_type)
            } else {
                0
            };
            let attacker_val = match self.game.squares[mv.from] {
                Square::Occupied(idx) => self.piece_value(self.game.pieces[idx].piece_type),
                Square::Empty => 0,
            };
            // MVV-LVA score: 10000 + victim * 10 - attacker
            return 100_000 + victim_val * 10 - attacker_val;
        }

        if let Some(promo) = mv.promotion {
            return 90_000 + self.piece_value(promo);
        }

        if ply < 64 {
            if Some(*mv) == self.killer_moves[ply][0] {
                return 80_000;
            }
            if Some(*mv) == self.killer_moves[ply][1] {
                return 70_000;
            }
        }

        0
    }

    fn piece_value(&self, pt: PieceType) -> i32 {
        match pt {
            PieceType::Pawn => 100,
            PieceType::Knight => 320,
            PieceType::Bishop => 330,
            PieceType::Rook => 500,
            PieceType::Queen => 900,
            PieceType::King => 20000,
        }
    }

    fn order_moves(&self, moves: &mut [Move], ply: usize, pv_move: Option<Move>) {
        moves.sort_by_cached_key(|mv| -self.score_move(mv, ply, pv_move));
    }

    fn order_captures(&self, moves: &mut [Move]) {
        moves.sort_by_cached_key(|mv| -self.score_move(mv, 0, None));
    }
}
