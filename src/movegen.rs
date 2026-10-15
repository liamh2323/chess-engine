use crate::board::Game;
use crate::types::*;

pub struct AttackTables {
    pub knight_attacks: [u64; 64],
    pub king_attacks: [u64; 64],
}

impl AttackTables {
    pub const fn init() -> Self {
        let mut knight_attacks = [0u64; 64];
        let mut king_attacks = [0u64; 64];

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

            sq += 1;
        }

        AttackTables {
            knight_attacks,
            king_attacks,
        }
    }
}

pub static ATTACKS: AttackTables = AttackTables::init();

const ROOK_DIRECTIONS: [(i32, i32); 4] = [(0, 1), (0, -1), (1, 0), (-1, 0)];
const BISHOP_DIRECTIONS: [(i32, i32); 4] = [(1, 1), (1, -1), (-1, 1), (-1, -1)];

impl Game {
    pub fn is_square_attacked(&self, sq: usize, by_colour: Colour) -> bool {
        if sq >= 64 {
            return false;
        }

        // 1. Knight attacks
        let knights = self.piece_bb(by_colour, PieceType::Knight);
        if ATTACKS.knight_attacks[sq] & knights != 0 {
            return true;
        }

        // 2. King attacks
        let kings = self.piece_bb(by_colour, PieceType::King);
        if ATTACKS.king_attacks[sq] & kings != 0 {
            return true;
        }

        // 3. Pawn attacks
        let file = sq % 8;
        let rank = sq / 8;
        match by_colour {
            Colour::White => {
                if rank > 0 {
                    if file > 0 {
                        let from_sq = sq - 9;
                        if let Square::Occupied(idx) = self.squares[from_sq] {
                            let p = &self.pieces[idx];
                            if p.colour == Colour::White && p.piece_type == PieceType::Pawn {
                                return true;
                            }
                        }
                    }
                    if file < 7 {
                        let from_sq = sq - 7;
                        if let Square::Occupied(idx) = self.squares[from_sq] {
                            let p = &self.pieces[idx];
                            if p.colour == Colour::White && p.piece_type == PieceType::Pawn {
                                return true;
                            }
                        }
                    }
                }
            }
            Colour::Black => {
                if rank < 7 {
                    if file > 0 {
                        let from_sq = sq + 7;
                        if let Square::Occupied(idx) = self.squares[from_sq] {
                            let p = &self.pieces[idx];
                            if p.colour == Colour::Black && p.piece_type == PieceType::Pawn {
                                return true;
                            }
                        }
                    }
                    if file < 7 {
                        let from_sq = sq + 9;
                        if let Square::Occupied(idx) = self.squares[from_sq] {
                            let p = &self.pieces[idx];
                            if p.colour == Colour::Black && p.piece_type == PieceType::Pawn {
                                return true;
                            }
                        }
                    }
                }
            }
        }

        // 4. Straight sliding attacks (Rooks and Queens)
        for &(df, dr) in &ROOK_DIRECTIONS {
            let mut f = file as i32 + df;
            let mut r = rank as i32 + dr;
            while (0..8).contains(&f) && (0..8).contains(&r) {
                let target_sq = (r * 8 + f) as usize;
                match self.squares[target_sq] {
                    Square::Empty => {}
                    Square::Occupied(idx) => {
                        let p = &self.pieces[idx];
                        if p.colour == by_colour
                            && (p.piece_type == PieceType::Rook || p.piece_type == PieceType::Queen)
                        {
                            return true;
                        }
                        break;
                    }
                }
                f += df;
                r += dr;
            }
        }

        // 5. Diagonal sliding attacks (Bishops and Queens)
        for &(df, dr) in &BISHOP_DIRECTIONS {
            let mut f = file as i32 + df;
            let mut r = rank as i32 + dr;
            while (0..8).contains(&f) && (0..8).contains(&r) {
                let target_sq = (r * 8 + f) as usize;
                match self.squares[target_sq] {
                    Square::Empty => {}
                    Square::Occupied(idx) => {
                        let p = &self.pieces[idx];
                        if p.colour == by_colour
                            && (p.piece_type == PieceType::Bishop
                                || p.piece_type == PieceType::Queen)
                        {
                            return true;
                        }
                        break;
                    }
                }
                f += df;
                r += dr;
            }
        }

        false
    }

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

        for piece in &self.pieces {
            if piece.position == 0 || piece.colour != us {
                continue;
            }

            let from_sq = bit_scan(piece.position);
            let file = from_sq % 8;
            let rank = from_sq / 8;

            match piece.piece_type {
                PieceType::Pawn => {
                    if us == Colour::White {
                        // Single step
                        let one_step = from_sq + 8;
                        if self.squares[one_step] == Square::Empty {
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
                                // Double step
                                if rank == 1 {
                                    let two_step = from_sq + 16;
                                    if self.squares[two_step] == Square::Empty {
                                        moves.push(Move::new(from_sq, two_step));
                                    }
                                }
                            }
                        }

                        // Left capture
                        if file > 0 {
                            let to_sq = from_sq + 7;
                            if let Square::Occupied(cap_idx) = self.squares[to_sq] {
                                if self.pieces[cap_idx].colour == them {
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
                                }
                            } else if self.en_passant == Some(1u64 << to_sq) {
                                moves.push(Move {
                                    from: from_sq,
                                    to: to_sq,
                                    promotion: None,
                                    is_en_passant: true,
                                    is_castling: false,
                                });
                            }
                        }

                        // Right capture
                        if file < 7 {
                            let to_sq = from_sq + 9;
                            if let Square::Occupied(cap_idx) = self.squares[to_sq] {
                                if self.pieces[cap_idx].colour == them {
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
                                }
                            } else if self.en_passant == Some(1u64 << to_sq) {
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
                        // Single step
                        let one_step = from_sq - 8;
                        if self.squares[one_step] == Square::Empty {
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
                                // Double step
                                if rank == 6 {
                                    let two_step = from_sq - 16;
                                    if self.squares[two_step] == Square::Empty {
                                        moves.push(Move::new(from_sq, two_step));
                                    }
                                }
                            }
                        }

                        // Left capture (file - 1)
                        if file > 0 {
                            let to_sq = from_sq - 9;
                            if let Square::Occupied(cap_idx) = self.squares[to_sq] {
                                if self.pieces[cap_idx].colour == them {
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
                                }
                            } else if self.en_passant == Some(1u64 << to_sq) {
                                moves.push(Move {
                                    from: from_sq,
                                    to: to_sq,
                                    promotion: None,
                                    is_en_passant: true,
                                    is_castling: false,
                                });
                            }
                        }

                        // Right capture (file + 1)
                        if file < 7 {
                            let to_sq = from_sq - 7;
                            if let Square::Occupied(cap_idx) = self.squares[to_sq] {
                                if self.pieces[cap_idx].colour == them {
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
                                }
                            } else if self.en_passant == Some(1u64 << to_sq) {
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

                PieceType::Knight => {
                    let mut targets = ATTACKS.knight_attacks[from_sq] & !friendly_occ;
                    while targets != 0 {
                        let to_sq = bit_scan(targets);
                        targets &= targets - 1;
                        moves.push(Move::new(from_sq, to_sq));
                    }
                }

                PieceType::King => {
                    let mut targets = ATTACKS.king_attacks[from_sq] & !friendly_occ;
                    while targets != 0 {
                        let to_sq = bit_scan(targets);
                        targets &= targets - 1;
                        moves.push(Move::new(from_sq, to_sq));
                    }

                    // Castling
                    if us == Colour::White && from_sq == 4 {
                        // Kingside
                        if self.castling_rights.contains(CastlingRights::WHITEKINGSIDE)
                            && self.squares[5] == Square::Empty
                            && self.squares[6] == Square::Empty
                            && !self.is_square_attacked(4, Colour::Black)
                            && !self.is_square_attacked(5, Colour::Black)
                            && !self.is_square_attacked(6, Colour::Black)
                            && let Square::Occupied(r_idx) = self.squares[7]
                        {
                            let r = &self.pieces[r_idx];
                            if r.colour == Colour::White && r.piece_type == PieceType::Rook {
                                moves.push(Move {
                                    from: 4,
                                    to: 6,
                                    promotion: None,
                                    is_en_passant: false,
                                    is_castling: true,
                                });
                            }
                        }

                        // Queenside
                        if self
                            .castling_rights
                            .contains(CastlingRights::WHITEQUEENSIDE)
                            && self.squares[1] == Square::Empty
                            && self.squares[2] == Square::Empty
                            && self.squares[3] == Square::Empty
                            && !self.is_square_attacked(4, Colour::Black)
                            && !self.is_square_attacked(3, Colour::Black)
                            && !self.is_square_attacked(2, Colour::Black)
                            && let Square::Occupied(r_idx) = self.squares[0]
                        {
                            let r = &self.pieces[r_idx];
                            if r.colour == Colour::White && r.piece_type == PieceType::Rook {
                                moves.push(Move {
                                    from: 4,
                                    to: 2,
                                    promotion: None,
                                    is_en_passant: false,
                                    is_castling: true,
                                });
                            }
                        }
                    } else if us == Colour::Black && from_sq == 60 {
                        // Kingside
                        if self.castling_rights.contains(CastlingRights::BLACKKINGSIDE)
                            && self.squares[61] == Square::Empty
                            && self.squares[62] == Square::Empty
                            && !self.is_square_attacked(60, Colour::White)
                            && !self.is_square_attacked(61, Colour::White)
                            && !self.is_square_attacked(62, Colour::White)
                            && let Square::Occupied(r_idx) = self.squares[63]
                        {
                            let r = &self.pieces[r_idx];
                            if r.colour == Colour::Black && r.piece_type == PieceType::Rook {
                                moves.push(Move {
                                    from: 60,
                                    to: 62,
                                    promotion: None,
                                    is_en_passant: false,
                                    is_castling: true,
                                });
                            }
                        }

                        // Queenside
                        if self
                            .castling_rights
                            .contains(CastlingRights::BLACKQUEENSIDE)
                            && self.squares[57] == Square::Empty
                            && self.squares[58] == Square::Empty
                            && self.squares[59] == Square::Empty
                            && !self.is_square_attacked(60, Colour::White)
                            && !self.is_square_attacked(59, Colour::White)
                            && !self.is_square_attacked(58, Colour::White)
                            && let Square::Occupied(r_idx) = self.squares[56]
                        {
                            let r = &self.pieces[r_idx];
                            if r.colour == Colour::Black && r.piece_type == PieceType::Rook {
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
                }

                PieceType::Bishop => {
                    self.generate_ray_moves(from_sq, &BISHOP_DIRECTIONS, them, &mut moves);
                }

                PieceType::Rook => {
                    self.generate_ray_moves(from_sq, &ROOK_DIRECTIONS, them, &mut moves);
                }

                PieceType::Queen => {
                    self.generate_ray_moves(from_sq, &ROOK_DIRECTIONS, them, &mut moves);
                    self.generate_ray_moves(from_sq, &BISHOP_DIRECTIONS, them, &mut moves);
                }
            }
        }

        moves
    }

    fn generate_ray_moves(
        &self,
        from_sq: usize,
        directions: &[(i32, i32)],
        enemy_colour: Colour,
        moves: &mut Vec<Move>,
    ) {
        let file = (from_sq % 8) as i32;
        let rank = (from_sq / 8) as i32;

        for &(df, dr) in directions {
            let mut f = file + df;
            let mut r = rank + dr;

            while (0..8).contains(&f) && (0..8).contains(&r) {
                let to_sq = (r * 8 + f) as usize;
                match self.squares[to_sq] {
                    Square::Empty => {
                        moves.push(Move::new(from_sq, to_sq));
                    }
                    Square::Occupied(idx) => {
                        if self.pieces[idx].colour == enemy_colour {
                            moves.push(Move::new(from_sq, to_sq));
                        }
                        break;
                    }
                }
                f += df;
                r += dr;
            }
        }
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
