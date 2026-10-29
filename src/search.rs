use std::ops::{Deref, DerefMut};
use std::time::Instant;

use crate::board::Game;
use crate::tt::{TTFlag, TranspositionTable};
use crate::types::*;

pub const INFINITY: i32 = 1_000_000;
pub const MATE_SCORE: i32 = 30_000;
pub const MAX_PLY: usize = 64;

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

pub enum TTRef<'a> {
    Owned(Box<TranspositionTable>),
    Borrowed(&'a mut TranspositionTable),
}

impl<'a> Deref for TTRef<'a> {
    type Target = TranspositionTable;
    fn deref(&self) -> &Self::Target {
        match self {
            TTRef::Owned(b) => b,
            TTRef::Borrowed(r) => r,
        }
    }
}

impl<'a> DerefMut for TTRef<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            TTRef::Owned(b) => b,
            TTRef::Borrowed(r) => r,
        }
    }
}

pub struct Searcher<'a> {
    pub game: &'a mut Game,
    pub tt: TTRef<'a>,
    pub nodes: u64,
    pub start_time: Instant,
    pub time_limit_ms: Option<u64>,
    pub soft_time_limit_ms: Option<u64>,
    pub node_limit: Option<u64>,
    pub mate_limit: Option<usize>,
    pub stopped: bool,
    pub killer_moves: [[Option<Move>; 2]; MAX_PLY],
    pub history: [[[i32; 64]; 64]; 2],
    pub counter_moves: Box<[[[Option<Move>; 64]; 64]; 2]>,
    pub pv_table: [[Option<Move>; MAX_PLY]; MAX_PLY],
    pub pv_length: [usize; MAX_PLY],
}

impl<'a> Searcher<'a> {
    pub fn new(game: &'a mut Game) -> Self {
        Searcher {
            game,
            tt: TTRef::Owned(Box::new(TranspositionTable::new_mb(16))),
            nodes: 0,
            start_time: Instant::now(),
            time_limit_ms: None,
            soft_time_limit_ms: None,
            node_limit: None,
            mate_limit: None,
            stopped: false,
            killer_moves: [[None; 2]; MAX_PLY],
            history: [[[0; 64]; 64]; 2],
            counter_moves: Box::new([[[None; 64]; 64]; 2]),
            pv_table: [[None; MAX_PLY]; MAX_PLY],
            pv_length: [0; MAX_PLY],
        }
    }

    pub fn with_tt(game: &'a mut Game, tt: &'a mut TranspositionTable) -> Self {
        Searcher {
            game,
            tt: TTRef::Borrowed(tt),
            nodes: 0,
            start_time: Instant::now(),
            time_limit_ms: None,
            soft_time_limit_ms: None,
            node_limit: None,
            mate_limit: None,
            stopped: false,
            killer_moves: [[None; 2]; MAX_PLY],
            history: [[[0; 64]; 64]; 2],
            counter_moves: Box::new([[[None; 64]; 64]; 2]),
            pv_table: [[None; MAX_PLY]; MAX_PLY],
            pv_length: [0; MAX_PLY],
        }
    }

    pub fn pv_line(&self) -> Vec<Move> {
        let mut line = Vec::with_capacity(self.pv_length[0]);
        for i in 0..self.pv_length[0] {
            if let Some(mv) = self.pv_table[0][i] {
                line.push(mv);
            } else {
                break;
            }
        }
        line
    }

    #[inline]
    fn check_limits(&mut self) {
        if let Some(max_nodes) = self.node_limit
            && self.nodes >= max_nodes
        {
            self.stopped = true;
            return;
        }
        if self.nodes.is_multiple_of(1024)
            && let Some(limit) = self.time_limit_ms
            && self.start_time.elapsed().as_millis() as u64 >= limit
        {
            self.stopped = true;
        }
    }

    #[inline]
    fn update_history(&mut self, c: usize, from: usize, to: usize, bonus: i32) {
        let clamped = bonus.clamp(-16_384, 16_384);
        let entry = &mut self.history[c][from][to];
        *entry += clamped - (*entry * clamped.abs()) / 16_384;
    }

    pub fn search(&mut self, limits: &SearchLimits) -> (Option<Move>, i32) {
        self.nodes = 0;
        self.start_time = Instant::now();
        self.time_limit_ms = limits.time_limit_ms;
        self.stopped = false;
        self.killer_moves = [[None; 2]; MAX_PLY];
        self.history = [[[0; 64]; 64]; 2];
        self.pv_length = [0; MAX_PLY];

        let mut best_move = None;
        let mut best_score = 0;

        let legal_moves = self.game.generate_legal_moves();
        if legal_moves.is_empty() {
            let in_check = self.game.is_in_check(self.game.active_colour);
            return (None, if in_check { -MATE_SCORE } else { 0 });
        }

        let max_depth = if legal_moves.len() == 1 && limits.time_limit_ms.is_some() {
            limits.max_depth.clamp(1, 4)
        } else {
            limits.max_depth.clamp(1, MAX_PLY - 1)
        };

        for depth in 1..=max_depth {
            let mut delta = 50;
            let mut alpha = if depth >= 4 {
                (best_score - delta).max(-INFINITY)
            } else {
                -INFINITY
            };
            let mut beta = if depth >= 4 {
                (best_score + delta).min(INFINITY)
            } else {
                INFINITY
            };

            let mut iter_best_move = None;
            let mut iter_best_score = -INFINITY;

            loop {
                let (mv, score) = self.search_root(depth, alpha, beta, &legal_moves, best_move);
                if self.stopped {
                    break;
                }

                if score <= alpha {
                    beta = (alpha + beta) / 2;
                    alpha = (score - delta).max(-INFINITY);
                    delta *= 2;
                } else if score >= beta {
                    if let Some(m) = mv {
                        iter_best_move = Some(m);
                        iter_best_score = score;
                    }
                    beta = (score + delta).min(INFINITY);
                    delta *= 2;
                } else {
                    iter_best_move = mv;
                    iter_best_score = score;
                    break;
                }

                if delta > 1000 {
                    alpha = -INFINITY;
                    beta = INFINITY;
                }
            }

            if self.stopped && depth > 1 && iter_best_move.is_none() {
                break;
            }

            if let Some(mv) = iter_best_move {
                best_move = Some(mv);
                best_score = iter_best_score;

                let elapsed_ms = self.start_time.elapsed().as_millis().max(1);
                let nps = (self.nodes as u128 * 1000) / elapsed_ms;
                let hashfull = self.tt.hashfull();

                let score_str = if best_score.abs() > MATE_SCORE - 500 {
                    let mate_plies = MATE_SCORE - best_score.abs();
                    let mate_moves = (mate_plies + 1) / 2;
                    let sign = if best_score > 0 { "" } else { "-" };
                    format!("mate {}{}", sign, mate_moves)
                } else {
                    format!("cp {}", best_score)
                };

                let pv_moves = self.pv_line();
                let pv_str = if pv_moves.is_empty() {
                    mv.to_uci()
                } else {
                    pv_moves
                        .iter()
                        .map(|m| m.to_uci())
                        .collect::<Vec<_>>()
                        .join(" ")
                };

                println!(
                    "info depth {} score {} time {} nodes {} nps {} hashfull {} pv {}",
                    depth, score_str, elapsed_ms, self.nodes, nps, hashfull, pv_str
                );

                if best_score.abs() > MATE_SCORE - 500 {
                    let mate_plies = (MATE_SCORE - best_score.abs()) as usize;
                    let mate_moves = mate_plies.div_ceil(2);
                    if let Some(target_mate) = self.mate_limit
                        && mate_moves <= target_mate
                    {
                        break;
                    }
                    if depth >= mate_plies {
                        // Found forced mate and searched to full mate depth
                        break;
                    }
                }
            }

            if self.stopped {
                break;
            }

            if let Some(soft_limit) = self.soft_time_limit_ms
                && self.start_time.elapsed().as_millis() as u64 >= soft_limit
            {
                break;
            }
        }

        (
            best_move.or_else(|| legal_moves.first().copied()),
            best_score,
        )
    }

    fn search_root(
        &mut self,
        depth: usize,
        mut alpha: i32,
        beta: i32,
        legal_moves: &[Move],
        prev_best: Option<Move>,
    ) -> (Option<Move>, i32) {
        let orig_alpha = alpha;
        let key = self.game.zobrist_hash;
        let tt_move = prev_best.or_else(|| self.tt.probe_move(key));

        let mut ordered_moves = legal_moves.to_vec();
        self.order_moves(&mut ordered_moves, 0, tt_move, None);

        let mut best_move = None;
        let mut best_score = -INFINITY;
        self.pv_length[0] = 0;

        for (i, mv) in ordered_moves.into_iter().enumerate() {
            let undo = self.game.make_move(&mv);

            let score = if i == 0 {
                -self.alpha_beta(depth - 1, -beta, -alpha, 1, true, Some(mv))
            } else {
                let mut s = -self.alpha_beta(depth - 1, -alpha - 1, -alpha, 1, true, Some(mv));
                if s > alpha && !self.stopped {
                    s = -self.alpha_beta(depth - 1, -beta, -alpha, 1, true, Some(mv));
                }
                s
            };

            self.game.unmake_move(&mv, &undo);

            if self.stopped {
                break;
            }

            if score > best_score {
                best_score = score;
                best_move = Some(mv);

                if score > alpha {
                    alpha = score;
                    self.update_pv(0, mv);
                }
            }

            if alpha >= beta {
                break;
            }
        }

        if !self.stopped
            && let Some(bm) = best_move
        {
            let flag = if best_score <= orig_alpha {
                TTFlag::UpperBound
            } else if best_score >= beta {
                TTFlag::LowerBound
            } else {
                TTFlag::Exact
            };
            self.tt.store(key, depth, flag, best_score, Some(bm), 0);
        }

        (best_move, best_score)
    }

    #[inline]
    fn update_pv(&mut self, ply: usize, mv: Move) {
        if ply >= MAX_PLY {
            return;
        }
        self.pv_table[ply][0] = Some(mv);
        let child_len = if ply + 1 < MAX_PLY {
            self.pv_length[ply + 1].min(MAX_PLY - 1)
        } else {
            0
        };
        for i in 0..child_len {
            self.pv_table[ply][i + 1] = self.pv_table[ply + 1][i];
        }
        self.pv_length[ply] = child_len + 1;
    }

    fn alpha_beta(
        &mut self,
        mut depth: usize,
        mut alpha: i32,
        mut beta: i32,
        ply: usize,
        can_null: bool,
        prev_move: Option<Move>,
    ) -> i32 {
        if ply < MAX_PLY {
            self.pv_length[ply] = 0;
        }

        // Draw detection (50-move rule, repetition, insufficient material)
        if ply > 0
            && (self.game.is_fifty_move_draw()
                || self.game.is_repetition()
                || self.game.is_insufficient_material())
        {
            return 0;
        }

        if ply >= MAX_PLY - 1 {
            return self.game.evaluate();
        }

        // Mate distance pruning
        alpha = alpha.max(-MATE_SCORE + ply as i32);
        beta = beta.min(MATE_SCORE - ply as i32 - 1);
        if alpha >= beta {
            return alpha;
        }

        let is_pv = beta - alpha > 1;
        let in_check = self.game.is_in_check(self.game.active_colour);

        if depth == 0 && !in_check {
            return self.quiescence(alpha, beta, ply);
        }

        self.nodes += 1;
        self.check_limits();
        if self.stopped {
            return 0;
        }

        // Check extension: if in check at depth 0, extend to depth 1 so we resolve check in alpha_beta
        if depth == 0 && in_check {
            depth = 1;
        }

        let key = self.game.zobrist_hash;
        let (tt_score, tt_move) = self.tt.probe(key, depth, alpha, beta, ply);
        if !is_pv
            && let Some(score) = tt_score
        {
            return score;
        }

        // Internal Iterative Reduction (IIR)
        if depth >= 4 && tt_move.is_none() && !in_check {
            depth -= 1;
        }

        let static_eval = if !in_check {
            self.game.evaluate()
        } else {
            -INFINITY
        };

        // Pruning at non-PV nodes when not in check
        if !is_pv && !in_check && beta.abs() < MATE_SCORE - 500 {
            // 1. Reverse Futility Pruning (Static Null Move Pruning)
            if depth <= 3 {
                let margin = 110 * depth as i32;
                if static_eval - margin >= beta {
                    return static_eval - margin;
                }
            }

            // 2. Null-Move Pruning
            if can_null
                && depth >= 3
                && static_eval >= beta
                && self.game.has_non_pawn_material(self.game.active_colour)
            {
                let r = if depth > 6 { 3 } else { 2 };
                let null_undo = self.game.make_null_move();
                let null_score = -self.alpha_beta(
                    depth.saturating_sub(1 + r),
                    -beta,
                    -beta + 1,
                    ply + 1,
                    false,
                    None,
                );
                self.game.unmake_null_move(&null_undo);

                if self.stopped {
                    return 0;
                }

                if null_score >= beta {
                    return if null_score > MATE_SCORE - 500 {
                        beta
                    } else {
                        null_score
                    };
                }
            }
        }

        let mut moves = self.game.generate_legal_moves();
        if moves.is_empty() {
            if in_check {
                return -MATE_SCORE + ply as i32;
            } else {
                return 0; // Stalemate
            }
        }

        self.order_moves(&mut moves, ply, tt_move, prev_move);

        let orig_alpha = alpha;
        let mut best_score = -INFINITY;
        let mut best_move = None;
        let us_idx = self.game.active_colour.index();
        let mut quiet_searched: Vec<Move> = Vec::with_capacity(16);

        let futility_prunable = !is_pv
            && !in_check
            && depth <= 3
            && alpha.abs() < MATE_SCORE - 500
            && static_eval + 120 * (depth as i32) <= alpha;

        for (i, mv) in moves.into_iter().enumerate() {
            let is_capture = self.game.squares[mv.to] != Square::Empty || mv.is_en_passant;
            let is_quiet = !is_capture && mv.promotion.is_none();

            // Late Move Pruning (LMP) & Futility Pruning on quiet moves after finding at least one move
            if i > 0 && best_score > -MATE_SCORE + 500 && is_quiet {
                // Late Move Pruning at shallow depths
                if !is_pv && !in_check && depth <= 3 && i >= 4 + depth * depth * 3 {
                    continue;
                }
            }

            // SEE pruning of bad captures at low depths
            let bad_capture = is_capture && !self.game.see_ge(&mv, 0);
            if i > 0
                && !is_pv
                && !in_check
                && bad_capture
                && depth <= 3
                && best_score > -MATE_SCORE + 500
                && !self.game.see_ge(&mv, -90 * (depth as i32))
            {
                continue;
            }

            let undo = self.game.make_move(&mv);
            let gives_check = self.game.is_in_check(self.game.active_colour);

            // Futility pruning (skip quiet moves that don't give check when far below alpha)
            if i > 0
                && futility_prunable
                && is_quiet
                && !gives_check
                && best_score > -MATE_SCORE + 500
            {
                self.game.unmake_move(&mv, &undo);
                continue;
            }

            let extension = if gives_check && ply < MAX_PLY - 2 {
                1
            } else {
                0
            };
            let new_depth = (depth - 1 + extension).min(MAX_PLY - 2);

            let score = if i == 0 {
                -self.alpha_beta(new_depth, -beta, -alpha, ply + 1, true, Some(mv))
            } else {
                // Late Move Reductions (LMR)
                let mut reduction: usize = 0;
                if depth >= 3 && i >= 3 && !in_check && !gives_check {
                    if is_quiet {
                        reduction = 1;
                        if i >= 8 && depth >= 5 {
                            reduction += 1;
                        }
                        if i >= 16 && depth >= 7 {
                            reduction += 1;
                        }
                        if is_pv {
                            reduction = reduction.saturating_sub(1);
                        }
                    } else if bad_capture {
                        reduction = 1;
                    }
                }

                let reduced_depth = new_depth.saturating_sub(reduction);
                let mut s = -self.alpha_beta(
                    reduced_depth,
                    -alpha - 1,
                    -alpha,
                    ply + 1,
                    true,
                    Some(mv),
                );

                if reduction > 0 && s > alpha && !self.stopped {
                    s = -self.alpha_beta(new_depth, -alpha - 1, -alpha, ply + 1, true, Some(mv));
                }

                if is_pv && s > alpha && !self.stopped {
                    s = -self.alpha_beta(new_depth, -beta, -alpha, ply + 1, true, Some(mv));
                }

                s
            };

            self.game.unmake_move(&mv, &undo);

            if self.stopped {
                return 0;
            }

            if score > best_score {
                best_score = score;
                best_move = Some(mv);
            }
            if score > alpha {
                alpha = score;
                self.update_pv(ply, mv);
            }
            if alpha >= beta {
                if is_quiet && ply < MAX_PLY {
                    if self.killer_moves[ply][0] != Some(mv) {
                        self.killer_moves[ply][1] = self.killer_moves[ply][0];
                        self.killer_moves[ply][0] = Some(mv);
                    }
                    let bonus = ((depth * depth) as i32).min(400);
                    self.update_history(us_idx, mv.from, mv.to, bonus * 32);
                    for prev_q in &quiet_searched {
                        self.update_history(us_idx, prev_q.from, prev_q.to, -bonus * 16);
                    }
                    if let Some(pm) = prev_move {
                        self.counter_moves[us_idx][pm.from][pm.to] = Some(mv);
                    }
                }
                break; // Beta cutoff
            }

            if is_quiet {
                quiet_searched.push(mv);
            }
        }

        let flag = if best_score <= orig_alpha {
            TTFlag::UpperBound
        } else if best_score >= beta {
            TTFlag::LowerBound
        } else {
            TTFlag::Exact
        };
        self.tt.store(key, depth, flag, best_score, best_move, ply);

        best_score
    }

    fn quiescence(&mut self, mut alpha: i32, beta: i32, ply: usize) -> i32 {
        self.nodes += 1;
        self.check_limits();
        if self.stopped {
            return 0;
        }

        if ply > 0
            && (self.game.is_fifty_move_draw()
                || self.game.is_repetition()
                || self.game.is_insufficient_material())
        {
            return 0;
        }

        if ply >= MAX_PLY - 1 {
            return self.game.evaluate();
        }

        let key = self.game.zobrist_hash;
        let is_pv = beta - alpha > 1;
        let (tt_score, tt_move) = self.tt.probe(key, 0, alpha, beta, ply);
        if !is_pv
            && let Some(score) = tt_score
        {
            return score;
        }

        let in_check = self.game.is_in_check(self.game.active_colour);
        let mut best_score = -INFINITY;
        let mut stand_pat = -INFINITY;

        if !in_check {
            stand_pat = self.game.evaluate();
            best_score = stand_pat;
            if best_score >= beta {
                return best_score;
            }
            if best_score > alpha {
                alpha = best_score;
            }
        }

        let us = self.game.active_colour;

        // If in check, examine all evasions; otherwise only captures and promotions
        let mut candidate_moves: Vec<Move> = if in_check {
            self.game.generate_pseudo_legal_moves()
        } else {
            self.game.generate_pseudo_legal_captures()
        };

        self.order_captures(&mut candidate_moves, tt_move);

        let mut legal_move_count = 0;
        for mv in candidate_moves {
            // When not in check, apply delta pruning and SEE pruning before making the move
            if !in_check && mv.promotion.is_none() {
                let victim_val = if mv.is_en_passant {
                    100
                } else if let Square::Occupied(idx) = self.game.squares[mv.to] {
                    self.piece_value(self.game.pieces[idx].piece_type)
                } else {
                    0
                };

                // Delta pruning: skip captures that cannot possibly raise alpha
                if stand_pat + victim_val + 200 < alpha {
                    continue;
                }

                // SEE pruning: skip captures that lose material
                if !self.game.see_ge(&mv, 0) {
                    continue;
                }
            }

            let undo = self.game.make_move(&mv);
            if self.game.is_in_check(us) {
                self.game.unmake_move(&mv, &undo);
                continue;
            }
            legal_move_count += 1;

            let score = -self.quiescence(-beta, -alpha, ply + 1);
            self.game.unmake_move(&mv, &undo);

            if self.stopped {
                return 0;
            }

            if score > best_score {
                best_score = score;
            }
            if score >= beta {
                return best_score;
            }
            if score > alpha {
                alpha = score;
            }
        }

        if in_check && legal_move_count == 0 {
            return -MATE_SCORE + ply as i32;
        }

        best_score
    }

    fn score_move(
        &self,
        mv: &Move,
        ply: usize,
        pv_move: Option<Move>,
        prev_move: Option<Move>,
    ) -> i32 {
        if Some(*mv) == pv_move {
            return 2_000_000;
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
            let promo_bonus = mv.promotion.map(|p| self.piece_value(p)).unwrap_or(0);
            let mvv_lva = victim_val * 10 + promo_bonus - attacker_val;
            // Winning/equal captures (SEE >= 0) are ordered above killers; losing captures below killers
            if victim_val >= attacker_val || self.game.see_ge(mv, 0) {
                return 200_000 + mvv_lva;
            } else {
                return 50_000 + mvv_lva;
            }
        }

        if let Some(promo) = mv.promotion {
            return 150_000 + self.piece_value(promo);
        }

        if ply < MAX_PLY {
            if Some(*mv) == self.killer_moves[ply][0] {
                return 120_000;
            }
            if Some(*mv) == self.killer_moves[ply][1] {
                return 110_000;
            }
        }

        let us_idx = self.game.active_colour.index();
        if let Some(pm) = prev_move
            && self.counter_moves[us_idx][pm.from][pm.to] == Some(*mv)
        {
            return 100_000;
        }

        self.history[us_idx][mv.from][mv.to]
    }

    #[inline]
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

    fn order_moves(
        &self,
        moves: &mut [Move],
        ply: usize,
        pv_move: Option<Move>,
        prev_move: Option<Move>,
    ) {
        moves.sort_unstable_by_key(|mv| -self.score_move(mv, ply, pv_move, prev_move));
    }

    fn order_captures(&self, moves: &mut [Move], tt_move: Option<Move>) {
        moves.sort_unstable_by_key(|mv| -self.score_move(mv, 0, tt_move, None));
    }
}
