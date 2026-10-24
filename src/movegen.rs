use crate::board::Game;
use crate::types::*;

pub const DIR_NORTH: usize = 0;
pub const DIR_EAST: usize = 1;
pub const DIR_NORTH_EAST: usize = 2;
pub const DIR_NORTH_WEST: usize = 3;
pub const DIR_SOUTH: usize = 4;
pub const DIR_WEST: usize = 5;
pub const DIR_SOUTH_EAST: usize = 6;
pub const DIR_SOUTH_WEST: usize = 7;

pub struct AttackTables {
    pub knight_attacks: [u64; 64],
    pub king_attacks: [u64; 64],
    pub pawn_attacks: [[u64; 64]; 2],
    pub ray_attacks: [[u64; 64]; 8],
}

impl AttackTables {
    pub const fn init() -> Self {
        let mut knight_attacks = [0u64; 64];
        let mut king_attacks = [0u64; 64];
        let mut pawn_attacks = [[0u64; 64]; 2];
        let mut ray_attacks = [[0u64; 64]; 8];

        let ray_deltas: [(i32, i32); 8] = [
            (0, 1),   // DIR_NORTH
            (1, 0),   // DIR_EAST
            (1, 1),   // DIR_NORTH_EAST
            (-1, 1),  // DIR_NORTH_WEST
            (0, -1),  // DIR_SOUTH
            (-1, 0),  // DIR_WEST
            (1, -1),  // DIR_SOUTH_EAST
            (-1, -1), // DIR_SOUTH_WEST
        ];

        let mut sq = 0;
        while sq < 64 {
            let file = sq % 8;
            let rank = sq / 8;

            // Knight
            let knight_deltas: [(i32, i32); 8] = [
                (-2, -1),
                (-2, 1),
                (-1, -2),
                (-1, 2),
                (1, -2),
                (1, 2),
                (2, -1),
                (2, 1),
            ];
            let mut k_mask = 0u64;
            let mut i = 0;
            while i < 8 {
                let (df, dr) = knight_deltas[i];
                let nf = file as i32 + df;
                let nr = rank as i32 + dr;
                if nf >= 0 && nf < 8 && nr >= 0 && nr < 8 {
                    k_mask |= 1u64 << (nr * 8 + nf);
                }
                i += 1;
            }
            knight_attacks[sq] = k_mask;

            // King
            let king_deltas: [(i32, i32); 8] = [
                (-1, -1),
                (-1, 0),
                (-1, 1),
                (0, -1),
                (0, 1),
                (1, -1),
                (1, 0),
                (1, 1),
            ];
            let mut kg_mask = 0u64;
            let mut j = 0;
            while j < 8 {
                let (df, dr) = king_deltas[j];
                let nf = file as i32 + df;
                let nr = rank as i32 + dr;
                if nf >= 0 && nf < 8 && nr >= 0 && nr < 8 {
                    kg_mask |= 1u64 << (nr * 8 + nf);
                }
                j += 1;
            }
            king_attacks[sq] = kg_mask;

            // Pawn attacks (White = 0, Black = 1)
            let mut wp_mask = 0u64;
            if rank < 7 {
                if file > 0 {
                    wp_mask |= 1u64 << (sq + 7);
                }
                if file < 7 {
                    wp_mask |= 1u64 << (sq + 9);
                }
            }
            pawn_attacks[0][sq] = wp_mask;

            let mut bp_mask = 0u64;
            if rank > 0 {
                if file > 0 {
                    bp_mask |= 1u64 << (sq - 9);
                }
                if file < 7 {
                    bp_mask |= 1u64 << (sq - 7);
                }
            }
            pawn_attacks[1][sq] = bp_mask;

            // Sliding rays
            let mut dir = 0;
            while dir < 8 {
                let (df, dr) = ray_deltas[dir];
                let mut f = file as i32 + df;
                let mut r = rank as i32 + dr;
                let mut r_mask = 0u64;
                while f >= 0 && f < 8 && r >= 0 && r < 8 {
                    r_mask |= 1u64 << (r * 8 + f);
                    f += df;
                    r += dr;
                }
                ray_attacks[dir][sq] = r_mask;
                dir += 1;
            }

            sq += 1;
        }

        AttackTables {
            knight_attacks,
            king_attacks,
            pawn_attacks,
            ray_attacks,
        }
    }
}

pub static ATTACKS: AttackTables = AttackTables::init();

#[inline]
fn positive_ray_attacks(dir: usize, sq: usize, occupied: u64) -> u64 {
    let attacks = ATTACKS.ray_attacks[dir][sq];
    let blockers = attacks & occupied;
    if blockers != 0 {
        let first_blocker = blockers.trailing_zeros() as usize;
        attacks ^ ATTACKS.ray_attacks[dir][first_blocker]
    } else {
        attacks
    }
}

#[inline]
fn negative_ray_attacks(dir: usize, sq: usize, occupied: u64) -> u64 {
    let attacks = ATTACKS.ray_attacks[dir][sq];
    let blockers = attacks & occupied;
    if blockers != 0 {
        let first_blocker = 63 - blockers.leading_zeros() as usize;
        attacks ^ ATTACKS.ray_attacks[dir][first_blocker]
    } else {
        attacks
    }
}

#[inline]
pub fn rook_attacks(sq: usize, occupied: u64) -> u64 {
    positive_ray_attacks(DIR_NORTH, sq, occupied)
        | positive_ray_attacks(DIR_EAST, sq, occupied)
        | negative_ray_attacks(DIR_SOUTH, sq, occupied)
        | negative_ray_attacks(DIR_WEST, sq, occupied)
}

#[inline]
pub fn bishop_attacks(sq: usize, occupied: u64) -> u64 {
    positive_ray_attacks(DIR_NORTH_EAST, sq, occupied)
        | positive_ray_attacks(DIR_NORTH_WEST, sq, occupied)
        | negative_ray_attacks(DIR_SOUTH_EAST, sq, occupied)
        | negative_ray_attacks(DIR_SOUTH_WEST, sq, occupied)
}

#[inline]
pub fn queen_attacks(sq: usize, occupied: u64) -> u64 {
    rook_attacks(sq, occupied) | bishop_attacks(sq, occupied)
}

impl Game {
    pub fn is_square_attacked(&self, sq: usize, by_colour: Colour) -> bool {
        if sq >= 64 {
            return false;
        }

        let opp_idx = by_colour.opponent().index();

        // 1. Pawn attacks
        if (ATTACKS.pawn_attacks[opp_idx][sq] & self.piece_bb(by_colour, PieceType::Pawn)) != 0 {
            return true;
        }

        // 2. Knight attacks
        if (ATTACKS.knight_attacks[sq] & self.piece_bb(by_colour, PieceType::Knight)) != 0 {
            return true;
        }

        // 3. King attacks
        if (ATTACKS.king_attacks[sq] & self.piece_bb(by_colour, PieceType::King)) != 0 {
            return true;
        }

        let occ = self.occupied_bb();
        let queens = self.piece_bb(by_colour, PieceType::Queen);

        // 4. Straight sliding attacks (Rooks and Queens)
        let rooks_queens = self.piece_bb(by_colour, PieceType::Rook) | queens;
        if rooks_queens != 0 && (rook_attacks(sq, occ) & rooks_queens) != 0 {
            return true;
        }

        // 5. Diagonal sliding attacks (Bishops and Queens)
        let bishops_queens = self.piece_bb(by_colour, PieceType::Bishop) | queens;
        if bishops_queens != 0 && (bishop_attacks(sq, occ) & bishops_queens) != 0 {
            return true;
        }

        false
    }

    #[inline]
    pub fn is_in_check(&self, colour: Colour) -> bool {
        let king_sq = self.king_square(colour);
        if king_sq >= 64 {
            return false;
        }
        self.is_square_attacked(king_sq, colour.opponent())
    }

    pub fn generate_pseudo_legal_moves(&self) -> Vec<Move> {
        let mut moves = Vec::with_capacity(64);
        let us = self.active_colour;
        let them = us.opponent();
        let friendly_occ = self.colour_bb(us);
        let enemy_occ = self.colour_bb(them);
        let occ = self.occupied_bb();

        // 1. Pawns
        let mut pawns = self.piece_bb(us, PieceType::Pawn);
        while pawns != 0 {
            let from_sq = bit_scan(pawns);
            pawns &= pawns - 1;
            let file = from_sq % 8;
            let rank = from_sq / 8;

            if us == Colour::White {
                let one_step = from_sq + 8;
                if (occ & (1u64 << one_step)) == 0 {
                    if rank == 6 {
                        for promo in [
                            PieceType::Queen,
                            PieceType::Rook,
                            PieceType::Bishop,
                            PieceType::Knight,
                        ] {
                            moves.push(Move {
                                from: from_sq,
                                to: one_step,
                                promotion: Some(promo),
                                is_en_passant: false,
                                is_castling: false,
                            });
                        }
                    } else {
                        moves.push(Move::new(from_sq, one_step));
                        if rank == 1 {
                            let two_step = from_sq + 16;
                            if (occ & (1u64 << two_step)) == 0 {
                                moves.push(Move::new(from_sq, two_step));
                            }
                        }
                    }
                }

                // Captures
                if file > 0 {
                    let to_sq = from_sq + 7;
                    let to_bit = 1u64 << to_sq;
                    if (enemy_occ & to_bit) != 0 {
                        if rank == 6 {
                            for promo in [
                                PieceType::Queen,
                                PieceType::Rook,
                                PieceType::Bishop,
                                PieceType::Knight,
                            ] {
                                moves.push(Move {
                                    from: from_sq,
                                    to: to_sq,
                                    promotion: Some(promo),
                                    is_en_passant: false,
                                    is_castling: false,
                                });
                            }
                        } else {
                            moves.push(Move::new(from_sq, to_sq));
                        }
                    } else if self.en_passant == Some(to_bit) {
                        moves.push(Move {
                            from: from_sq,
                            to: to_sq,
                            promotion: None,
                            is_en_passant: true,
                            is_castling: false,
                        });
                    }
                }

                if file < 7 {
                    let to_sq = from_sq + 9;
                    let to_bit = 1u64 << to_sq;
                    if (enemy_occ & to_bit) != 0 {
                        if rank == 6 {
                            for promo in [
                                PieceType::Queen,
                                PieceType::Rook,
                                PieceType::Bishop,
                                PieceType::Knight,
                            ] {
                                moves.push(Move {
                                    from: from_sq,
                                    to: to_sq,
                                    promotion: Some(promo),
                                    is_en_passant: false,
                                    is_castling: false,
                                });
                            }
                        } else {
                            moves.push(Move::new(from_sq, to_sq));
                        }
                    } else if self.en_passant == Some(to_bit) {
                        moves.push(Move {
                            from: from_sq,
                            to: to_sq,
                            promotion: None,
                            is_en_passant: true,
                            is_castling: false,
                        });
                    }
                }
            } else {
                // Black pawns
                let one_step = from_sq - 8;
                if (occ & (1u64 << one_step)) == 0 {
                    if rank == 1 {
                        for promo in [
                            PieceType::Queen,
                            PieceType::Rook,
                            PieceType::Bishop,
                            PieceType::Knight,
                        ] {
                            moves.push(Move {
                                from: from_sq,
                                to: one_step,
                                promotion: Some(promo),
                                is_en_passant: false,
                                is_castling: false,
                            });
                        }
                    } else {
                        moves.push(Move::new(from_sq, one_step));
                        if rank == 6 {
                            let two_step = from_sq - 16;
                            if (occ & (1u64 << two_step)) == 0 {
                                moves.push(Move::new(from_sq, two_step));
                            }
                        }
                    }
                }

                // Captures
                if file > 0 {
                    let to_sq = from_sq - 9;
                    let to_bit = 1u64 << to_sq;
                    if (enemy_occ & to_bit) != 0 {
                        if rank == 1 {
                            for promo in [
                                PieceType::Queen,
                                PieceType::Rook,
                                PieceType::Bishop,
                                PieceType::Knight,
                            ] {
                                moves.push(Move {
                                    from: from_sq,
                                    to: to_sq,
                                    promotion: Some(promo),
                                    is_en_passant: false,
                                    is_castling: false,
                                });
                            }
                        } else {
                            moves.push(Move::new(from_sq, to_sq));
                        }
                    } else if self.en_passant == Some(to_bit) {
                        moves.push(Move {
                            from: from_sq,
                            to: to_sq,
                            promotion: None,
                            is_en_passant: true,
                            is_castling: false,
                        });
                    }
                }

                if file < 7 {
                    let to_sq = from_sq - 7;
                    let to_bit = 1u64 << to_sq;
                    if (enemy_occ & to_bit) != 0 {
                        if rank == 1 {
                            for promo in [
                                PieceType::Queen,
                                PieceType::Rook,
                                PieceType::Bishop,
                                PieceType::Knight,
                            ] {
                                moves.push(Move {
                                    from: from_sq,
                                    to: to_sq,
                                    promotion: Some(promo),
                                    is_en_passant: false,
                                    is_castling: false,
                                });
                            }
                        } else {
                            moves.push(Move::new(from_sq, to_sq));
                        }
                    } else if self.en_passant == Some(to_bit) {
                        moves.push(Move {
                            from: from_sq,
                            to: to_sq,
                            promotion: None,
                            is_en_passant: true,
                            is_castling: false,
                        });
                    }
                }
            }
        }

        // 2. Knights
        let mut knights = self.piece_bb(us, PieceType::Knight);
        while knights != 0 {
            let from_sq = bit_scan(knights);
            knights &= knights - 1;
            let mut targets = ATTACKS.knight_attacks[from_sq] & !friendly_occ;
            while targets != 0 {
                let to_sq = bit_scan(targets);
                targets &= targets - 1;
                moves.push(Move::new(from_sq, to_sq));
            }
        }

        // 3. Bishops
        let mut bishops = self.piece_bb(us, PieceType::Bishop);
        while bishops != 0 {
            let from_sq = bit_scan(bishops);
            bishops &= bishops - 1;
            let mut targets = bishop_attacks(from_sq, occ) & !friendly_occ;
            while targets != 0 {
                let to_sq = bit_scan(targets);
                targets &= targets - 1;
                moves.push(Move::new(from_sq, to_sq));
            }
        }

        // 4. Rooks
        let mut rooks = self.piece_bb(us, PieceType::Rook);
        while rooks != 0 {
            let from_sq = bit_scan(rooks);
            rooks &= rooks - 1;
            let mut targets = rook_attacks(from_sq, occ) & !friendly_occ;
            while targets != 0 {
                let to_sq = bit_scan(targets);
                targets &= targets - 1;
                moves.push(Move::new(from_sq, to_sq));
            }
        }

        // 5. Queens
        let mut queens = self.piece_bb(us, PieceType::Queen);
        while queens != 0 {
            let from_sq = bit_scan(queens);
            queens &= queens - 1;
            let mut targets = queen_attacks(from_sq, occ) & !friendly_occ;
            while targets != 0 {
                let to_sq = bit_scan(targets);
                targets &= targets - 1;
                moves.push(Move::new(from_sq, to_sq));
            }
        }

        // 6. King
        let mut kings = self.piece_bb(us, PieceType::King);
        while kings != 0 {
            let from_sq = bit_scan(kings);
            kings &= kings - 1;
            let mut targets = ATTACKS.king_attacks[from_sq] & !friendly_occ;
            while targets != 0 {
                let to_sq = bit_scan(targets);
                targets &= targets - 1;
                moves.push(Move::new(from_sq, to_sq));
            }

            // Castling
            let our_rooks = self.piece_bb(us, PieceType::Rook);
            if us == Colour::White && from_sq == 4 {
                if self.castling_rights.contains(CastlingRights::WHITEKINGSIDE)
                    && (occ & ((1u64 << 5) | (1u64 << 6))) == 0
                    && (our_rooks & (1u64 << 7)) != 0
                    && !self.is_square_attacked(4, Colour::Black)
                    && !self.is_square_attacked(5, Colour::Black)
                    && !self.is_square_attacked(6, Colour::Black)
                {
                    moves.push(Move {
                        from: 4,
                        to: 6,
                        promotion: None,
                        is_en_passant: false,
                        is_castling: true,
                    });
                }

                if self
                    .castling_rights
                    .contains(CastlingRights::WHITEQUEENSIDE)
                    && (occ & ((1u64 << 1) | (1u64 << 2) | (1u64 << 3))) == 0
                    && (our_rooks & (1u64 << 0)) != 0
                    && !self.is_square_attacked(4, Colour::Black)
                    && !self.is_square_attacked(3, Colour::Black)
                    && !self.is_square_attacked(2, Colour::Black)
                {
                    moves.push(Move {
                        from: 4,
                        to: 2,
                        promotion: None,
                        is_en_passant: false,
                        is_castling: true,
                    });
                }
            } else if us == Colour::Black && from_sq == 60 {
                if self.castling_rights.contains(CastlingRights::BLACKKINGSIDE)
                    && (occ & ((1u64 << 61) | (1u64 << 62))) == 0
                    && (our_rooks & (1u64 << 63)) != 0
                    && !self.is_square_attacked(60, Colour::White)
                    && !self.is_square_attacked(61, Colour::White)
                    && !self.is_square_attacked(62, Colour::White)
                {
                    moves.push(Move {
                        from: 60,
                        to: 62,
                        promotion: None,
                        is_en_passant: false,
                        is_castling: true,
                    });
                }

                if self
                    .castling_rights
                    .contains(CastlingRights::BLACKQUEENSIDE)
                    && (occ & ((1u64 << 57) | (1u64 << 58) | (1u64 << 59))) == 0
                    && (our_rooks & (1u64 << 56)) != 0
                    && !self.is_square_attacked(60, Colour::White)
                    && !self.is_square_attacked(59, Colour::White)
                    && !self.is_square_attacked(58, Colour::White)
                {
                    moves.push(Move {
                        from: 60,
                        to: 58,
                        promotion: None,
                        is_en_passant: false,
                        is_castling: true,
                    });
                }
            }
        }

        moves
    }

    pub fn generate_legal_moves(&mut self) -> Vec<Move> {
        let pseudo_moves = self.generate_pseudo_legal_moves();
        let us = self.active_colour;
        let mut legal_moves = Vec::with_capacity(pseudo_moves.len());

        for mv in pseudo_moves {
            let undo = self.make_move(&mv);
            if !self.is_in_check(us) {
                legal_moves.push(mv);
            }
            self.unmake_move(&mv, &undo);
        }

        legal_moves
    }

    pub fn perft(&mut self, depth: usize) -> u64 {
        if depth == 0 {
            return 1;
        }
        let moves = self.generate_legal_moves();
        if depth == 1 {
            return moves.len() as u64;
        }

        let mut nodes = 0;
        for mv in moves {
            let undo = self.make_move(&mv);
            nodes += self.perft(depth - 1);
            self.unmake_move(&mv, &undo);
        }
        nodes
    }

    pub fn perft_divide(&mut self, depth: usize) -> Vec<(Move, u64)> {
        if depth == 0 {
            return vec![];
        }
        let moves = self.generate_legal_moves();
        let mut results = Vec::new();

        for mv in moves {
            let undo = self.make_move(&mv);
            let count = if depth == 1 { 1 } else { self.perft(depth - 1) };
            self.unmake_move(&mv, &undo);
            results.push((mv, count));
        }

        results
    }
}
