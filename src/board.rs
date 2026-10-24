use crate::types::*;
use crate::zobrist::ZOBRIST;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UndoInfo {
    pub moved_piece_idx: usize,
    pub moved_piece_orig: Piece,
    pub captured: Option<(usize, Piece)>,
    pub ep_captured: Option<(usize, usize, Piece)>, // (square, idx, piece)
    pub castling_rights: CastlingRights,
    pub en_passant: Option<PiecePosition>,
    pub halfmove_clock: usize,
    pub zobrist_hash: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NullUndoInfo {
    pub en_passant: Option<PiecePosition>,
    pub halfmove_clock: usize,
    pub zobrist_hash: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Game {
    pub pieces: Vec<Piece>,
    pub squares: Vec<Square>,
    pub active_colour: Colour,
    pub castling_rights: CastlingRights,
    pub en_passant: Option<PiecePosition>,
    pub halfmove_clock: usize,
    pub fullmove_clock: usize,
    pub piece_bbs: [[u64; 6]; 2],
    pub colour_bbs: [u64; 2],
    pub occupied: u64,
    pub zobrist_hash: u64,
    pub history: Vec<u64>,
}

impl Game {
    pub fn push_piece_and_square(
        &mut self,
        position: usize,
        colour: Colour,
        piece_type: PieceType,
        index: &mut usize,
    ) {
        let bit = 1u64 << position;
        self.pieces.push(Piece {
            position: bit,
            colour,
            piece_type,
        });
        self.squares.push(Square::Occupied(*index));
        self.piece_bbs[colour.index()][piece_type.index()] |= bit;
        self.colour_bbs[colour.index()] |= bit;
        self.occupied |= bit;
        self.zobrist_hash ^= ZOBRIST.piece_key(colour, piece_type, position);
        *index += 1;
    }

    pub fn push_empty_square(&mut self) {
        self.squares.push(Square::Empty);
    }

    pub fn compute_zobrist(&self) -> u64 {
        let mut hash = 0u64;
        for p in &self.pieces {
            if p.position != 0 {
                let sq = bit_scan(p.position);
                hash ^= ZOBRIST.piece_key(p.colour, p.piece_type, sq);
            }
        }
        hash ^= ZOBRIST.castling_key(self.castling_rights);
        if let Some(ep_bit) = self.en_passant {
            hash ^= ZOBRIST.ep_key(bit_scan(ep_bit));
        }
        if self.active_colour == Colour::Black {
            hash ^= ZOBRIST.side_to_move;
        }
        hash
    }

    pub fn initialise() -> Game {
        let mut game = Game {
            pieces: Vec::with_capacity(32),
            squares: Vec::with_capacity(64),
            active_colour: Colour::White,
            castling_rights: CastlingRights::ALL,
            en_passant: None,
            halfmove_clock: 0,
            fullmove_clock: 1,
            piece_bbs: [[0u64; 6]; 2],
            colour_bbs: [0u64; 2],
            occupied: 0u64,
            zobrist_hash: ZOBRIST.castling_key(CastlingRights::ALL),
            history: Vec::with_capacity(128),
        };
        let mut piece_index = 0;
        let colour = Colour::White;

        game.push_piece_and_square(0, colour, PieceType::Rook, &mut piece_index);
        game.push_piece_and_square(1, colour, PieceType::Knight, &mut piece_index);
        game.push_piece_and_square(2, colour, PieceType::Bishop, &mut piece_index);
        game.push_piece_and_square(3, colour, PieceType::Queen, &mut piece_index);
        game.push_piece_and_square(4, colour, PieceType::King, &mut piece_index);
        game.push_piece_and_square(5, colour, PieceType::Bishop, &mut piece_index);
        game.push_piece_and_square(6, colour, PieceType::Knight, &mut piece_index);
        game.push_piece_and_square(7, colour, PieceType::Rook, &mut piece_index);

        for i in 8..16 {
            game.push_piece_and_square(i, colour, PieceType::Pawn, &mut piece_index);
        }
        for _ in 16..48 {
            game.push_empty_square();
        }

        let colour = Colour::Black;
        for i in 48..56 {
            game.push_piece_and_square(i, colour, PieceType::Pawn, &mut piece_index);
        }

        let offset = 56;
        game.push_piece_and_square(offset, colour, PieceType::Rook, &mut piece_index);
        game.push_piece_and_square(1 + offset, colour, PieceType::Knight, &mut piece_index);
        game.push_piece_and_square(2 + offset, colour, PieceType::Bishop, &mut piece_index);
        game.push_piece_and_square(3 + offset, colour, PieceType::Queen, &mut piece_index);
        game.push_piece_and_square(4 + offset, colour, PieceType::King, &mut piece_index);
        game.push_piece_and_square(5 + offset, colour, PieceType::Bishop, &mut piece_index);
        game.push_piece_and_square(6 + offset, colour, PieceType::Knight, &mut piece_index);
        game.push_piece_and_square(7 + offset, colour, PieceType::Rook, &mut piece_index);

        game
    }

    #[inline]
    pub fn occupied_bb(&self) -> u64 {
        self.occupied
    }

    #[inline]
    pub fn colour_bb(&self, colour: Colour) -> u64 {
        self.colour_bbs[colour.index()]
    }

    #[inline]
    pub fn piece_bb(&self, colour: Colour, piece_type: PieceType) -> u64 {
        self.piece_bbs[colour.index()][piece_type.index()]
    }

    #[inline]
    pub fn king_square(&self, colour: Colour) -> usize {
        bit_scan(self.piece_bbs[colour.index()][PieceType::King.index()])
    }

    #[inline]
    pub fn has_non_pawn_material(&self, colour: Colour) -> bool {
        let c = colour.index();
        (self.piece_bbs[c][PieceType::Knight.index()]
            | self.piece_bbs[c][PieceType::Bishop.index()]
            | self.piece_bbs[c][PieceType::Rook.index()]
            | self.piece_bbs[c][PieceType::Queen.index()])
            != 0
    }

    pub fn repetition_count(&self) -> usize {
        let len = self.history.len();
        if len < 2 {
            return 0;
        }
        let max_back = self.halfmove_clock.min(len);
        let mut count = 0;
        let mut back = 2;
        while back <= max_back {
            if self.history[len - back] == self.zobrist_hash {
                count += 1;
            }
            back += 2;
        }
        count
    }

    #[inline]
    pub fn is_repetition(&self) -> bool {
        self.repetition_count() >= 1
    }

    #[inline]
    pub fn is_threefold_repetition(&self) -> bool {
        self.repetition_count() >= 2
    }

    #[inline]
    pub fn is_fifty_move_draw(&self) -> bool {
        self.halfmove_clock >= 100
    }

    pub fn is_insufficient_material(&self) -> bool {
        let pawns = self.piece_bbs[0][PieceType::Pawn.index()]
            | self.piece_bbs[1][PieceType::Pawn.index()];
        let rooks_queens = self.piece_bbs[0][PieceType::Rook.index()]
            | self.piece_bbs[1][PieceType::Rook.index()]
            | self.piece_bbs[0][PieceType::Queen.index()]
            | self.piece_bbs[1][PieceType::Queen.index()];

        if (pawns | rooks_queens) != 0 {
            return false;
        }

        let w_knights = self.piece_bbs[0][PieceType::Knight.index()];
        let b_knights = self.piece_bbs[1][PieceType::Knight.index()];
        let w_bishops = self.piece_bbs[0][PieceType::Bishop.index()];
        let b_bishops = self.piece_bbs[1][PieceType::Bishop.index()];

        let minor_count = (w_knights | b_knights | w_bishops | b_bishops).count_ones();
        if minor_count <= 1 {
            return true;
        }

        // KB vs KB with bishops on the same square colour
        if minor_count == 2
            && (w_knights | b_knights) == 0
            && w_bishops.count_ones() == 1
            && b_bishops.count_ones() == 1
        {
            const LIGHT_SQUARES: u64 = 0x55AA_55AA_55AA_55AA;
            let w_light = (w_bishops & LIGHT_SQUARES) != 0;
            let b_light = (b_bishops & LIGHT_SQUARES) != 0;
            if w_light == b_light {
                return true;
            }
        }

        false
    }

    pub fn is_draw(&self) -> bool {
        self.is_fifty_move_draw()
            || self.is_threefold_repetition()
            || self.is_insufficient_material()
    }

    pub fn make_null_move(&mut self) -> NullUndoInfo {
        let undo = NullUndoInfo {
            en_passant: self.en_passant,
            halfmove_clock: self.halfmove_clock,
            zobrist_hash: self.zobrist_hash,
        };
        self.history.push(self.zobrist_hash);

        if let Some(ep_bit) = self.en_passant {
            self.zobrist_hash ^= ZOBRIST.ep_key(bit_scan(ep_bit));
            self.en_passant = None;
        }

        self.halfmove_clock += 1;
        self.active_colour = self.active_colour.opponent();
        self.zobrist_hash ^= ZOBRIST.side_to_move;
        undo
    }

    pub fn unmake_null_move(&mut self, undo: &NullUndoInfo) {
        let _ = self.history.pop();
        self.active_colour = self.active_colour.opponent();
        self.en_passant = undo.en_passant;
        self.halfmove_clock = undo.halfmove_clock;
        self.zobrist_hash = undo.zobrist_hash;
    }

    pub fn make_move(&mut self, mv: &Move) -> UndoInfo {
        let moved_piece_idx = match self.squares[mv.from] {
            Square::Occupied(idx) => idx,
            Square::Empty => panic!("Illegal move from empty square: {}", mv.from),
        };
        let moved_piece_orig = self.pieces[moved_piece_idx];
        let us = self.active_colour;
        let them = us.opponent();
        let us_idx = us.index();
        let them_idx = them.index();

        let orig_hash = self.zobrist_hash;
        self.history.push(orig_hash);

        let mut hash = orig_hash;
        hash ^= ZOBRIST.castling_key(self.castling_rights);
        if let Some(ep_bit) = self.en_passant {
            hash ^= ZOBRIST.ep_key(bit_scan(ep_bit));
        }

        let mut captured = None;
        let mut ep_captured = None;

        if mv.is_en_passant {
            let cap_sq = if us == Colour::White {
                mv.to - 8
            } else {
                mv.to + 8
            };
            let cap_idx = match self.squares[cap_sq] {
                Square::Occupied(idx) => idx,
                Square::Empty => panic!("En passant capture on empty square: {}", cap_sq),
            };
            let cap_bit = 1u64 << cap_sq;
            ep_captured = Some((cap_sq, cap_idx, self.pieces[cap_idx]));
            self.pieces[cap_idx].position = 0;
            self.squares[cap_sq] = Square::Empty;

            self.piece_bbs[them_idx][PieceType::Pawn.index()] &= !cap_bit;
            self.colour_bbs[them_idx] &= !cap_bit;
            self.occupied &= !cap_bit;
            hash ^= ZOBRIST.piece_key(them, PieceType::Pawn, cap_sq);
        } else if let Square::Occupied(cap_idx) = self.squares[mv.to] {
            let cap_piece = self.pieces[cap_idx];
            let cap_bit = 1u64 << mv.to;
            captured = Some((cap_idx, cap_piece));
            self.pieces[cap_idx].position = 0;

            self.piece_bbs[them_idx][cap_piece.piece_type.index()] &= !cap_bit;
            self.colour_bbs[them_idx] &= !cap_bit;
            self.occupied &= !cap_bit;
            hash ^= ZOBRIST.piece_key(them, cap_piece.piece_type, mv.to);
        }

        let undo = UndoInfo {
            moved_piece_idx,
            moved_piece_orig,
            captured,
            ep_captured,
            castling_rights: self.castling_rights,
            en_passant: self.en_passant,
            halfmove_clock: self.halfmove_clock,
            zobrist_hash: orig_hash,
        };

        // Move the piece
        let from_bit = 1u64 << mv.from;
        let to_bit = 1u64 << mv.to;
        let orig_pt = moved_piece_orig.piece_type;
        let final_pt = mv.promotion.unwrap_or(orig_pt);

        self.squares[mv.from] = Square::Empty;
        self.squares[mv.to] = Square::Occupied(moved_piece_idx);
        self.pieces[moved_piece_idx].position = to_bit;
        self.pieces[moved_piece_idx].piece_type = final_pt;

        self.piece_bbs[us_idx][orig_pt.index()] &= !from_bit;
        self.piece_bbs[us_idx][final_pt.index()] |= to_bit;
        self.colour_bbs[us_idx] = (self.colour_bbs[us_idx] & !from_bit) | to_bit;
        self.occupied = (self.occupied & !from_bit) | to_bit;

        hash ^= ZOBRIST.piece_key(us, orig_pt, mv.from);
        hash ^= ZOBRIST.piece_key(us, final_pt, mv.to);

        // Handle castling rook movement
        if mv.is_castling {
            let (r_from, r_to) = match mv.to {
                6 => (7, 5),
                2 => (0, 3),
                62 => (63, 61),
                58 => (56, 59),
                _ => panic!("Invalid castling destination: {}", mv.to),
            };
            let r_idx = match self.squares[r_from] {
                Square::Occupied(i) => i,
                _ => panic!("Rook missing at {} for castling", r_from),
            };
            let r_from_bit = 1u64 << r_from;
            let r_to_bit = 1u64 << r_to;

            self.squares[r_from] = Square::Empty;
            self.squares[r_to] = Square::Occupied(r_idx);
            self.pieces[r_idx].position = r_to_bit;

            self.piece_bbs[us_idx][PieceType::Rook.index()] =
                (self.piece_bbs[us_idx][PieceType::Rook.index()] & !r_from_bit) | r_to_bit;
            self.colour_bbs[us_idx] = (self.colour_bbs[us_idx] & !r_from_bit) | r_to_bit;
            self.occupied = (self.occupied & !r_from_bit) | r_to_bit;

            hash ^= ZOBRIST.piece_key(us, PieceType::Rook, r_from);
            hash ^= ZOBRIST.piece_key(us, PieceType::Rook, r_to);
        }

        // Update en-passant square
        if orig_pt == PieceType::Pawn && (mv.to as isize - mv.from as isize).abs() == 16 {
            let ep_sq = (mv.from + mv.to) / 2;
            self.en_passant = Some(1u64 << ep_sq);
            hash ^= ZOBRIST.ep_key(ep_sq);
        } else {
            self.en_passant = None;
        }

        // Update castling rights
        self.castling_rights &= Self::castling_mask(mv.from) & Self::castling_mask(mv.to);
        hash ^= ZOBRIST.castling_key(self.castling_rights);

        // Update clocks
        if orig_pt == PieceType::Pawn || captured.is_some() || ep_captured.is_some() {
            self.halfmove_clock = 0;
        } else {
            self.halfmove_clock += 1;
        }

        if us == Colour::Black {
            self.fullmove_clock += 1;
        }

        self.active_colour = them;
        hash ^= ZOBRIST.side_to_move;
        self.zobrist_hash = hash;

        undo
    }

    pub fn unmake_move(&mut self, mv: &Move, undo: &UndoInfo) {
        let _ = self.history.pop();
        self.active_colour = self.active_colour.opponent();
        let us = self.active_colour;
        let them = us.opponent();
        let us_idx = us.index();
        let them_idx = them.index();

        if us == Colour::Black {
            self.fullmove_clock -= 1;
        }
        self.halfmove_clock = undo.halfmove_clock;
        self.castling_rights = undo.castling_rights;
        self.en_passant = undo.en_passant;
        self.zobrist_hash = undo.zobrist_hash;

        // Undo castling rook movement
        if mv.is_castling {
            let (r_from, r_to) = match mv.to {
                6 => (7, 5),
                2 => (0, 3),
                62 => (63, 61),
                58 => (56, 59),
                _ => panic!("Invalid castling destination: {}", mv.to),
            };
            let r_idx = match self.squares[r_to] {
                Square::Occupied(i) => i,
                _ => panic!("Rook missing at {}", r_to),
            };
            let r_from_bit = 1u64 << r_from;
            let r_to_bit = 1u64 << r_to;

            self.squares[r_to] = Square::Empty;
            self.squares[r_from] = Square::Occupied(r_idx);
            self.pieces[r_idx].position = r_from_bit;

            self.piece_bbs[us_idx][PieceType::Rook.index()] =
                (self.piece_bbs[us_idx][PieceType::Rook.index()] & !r_to_bit) | r_from_bit;
            self.colour_bbs[us_idx] = (self.colour_bbs[us_idx] & !r_to_bit) | r_from_bit;
            self.occupied = (self.occupied & !r_to_bit) | r_from_bit;
        }

        // Restore moved piece
        let moved_idx = undo.moved_piece_idx;
        let orig_pt = undo.moved_piece_orig.piece_type;
        let final_pt = mv.promotion.unwrap_or(orig_pt);
        let from_bit = 1u64 << mv.from;
        let to_bit = 1u64 << mv.to;

        self.pieces[moved_idx] = undo.moved_piece_orig;
        self.squares[mv.to] = Square::Empty;
        self.squares[mv.from] = Square::Occupied(moved_idx);

        self.piece_bbs[us_idx][final_pt.index()] &= !to_bit;
        self.piece_bbs[us_idx][orig_pt.index()] |= from_bit;
        self.colour_bbs[us_idx] = (self.colour_bbs[us_idx] & !to_bit) | from_bit;
        self.occupied = (self.occupied & !to_bit) | from_bit;

        // Restore captured pieces
        if let Some((cap_sq, cap_idx, orig_piece)) = undo.ep_captured {
            let cap_bit = 1u64 << cap_sq;
            self.pieces[cap_idx] = orig_piece;
            self.squares[cap_sq] = Square::Occupied(cap_idx);
            self.piece_bbs[them_idx][PieceType::Pawn.index()] |= cap_bit;
            self.colour_bbs[them_idx] |= cap_bit;
            self.occupied |= cap_bit;
        } else if let Some((cap_idx, orig_piece)) = undo.captured {
            self.pieces[cap_idx] = orig_piece;
            self.squares[mv.to] = Square::Occupied(cap_idx);
            self.piece_bbs[them_idx][orig_piece.piece_type.index()] |= to_bit;
            self.colour_bbs[them_idx] |= to_bit;
            self.occupied |= to_bit;
        }
    }

    fn castling_mask(sq: usize) -> CastlingRights {
        match sq {
            0 => !CastlingRights::WHITEQUEENSIDE,
            4 => !(CastlingRights::WHITEKINGSIDE | CastlingRights::WHITEQUEENSIDE),
            7 => !CastlingRights::WHITEKINGSIDE,
            56 => !CastlingRights::BLACKQUEENSIDE,
            60 => !(CastlingRights::BLACKKINGSIDE | CastlingRights::BLACKQUEENSIDE),
            63 => !CastlingRights::BLACKKINGSIDE,
            _ => CastlingRights::all(),
        }
    }
}

impl fmt::Display for Game {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "  +-----------------+")?;
        for rank in (0..8).rev() {
            write!(f, "{} | ", rank + 1)?;
            for file in 0..8 {
                let sq = rank * 8 + file;
                match self.squares[sq] {
                    Square::Empty => write!(f, ". ")?,
                    Square::Occupied(idx) => {
                        let p = &self.pieces[idx];
                        write!(f, "{}", p)?;
                    }
                }
            }
            writeln!(f, "|")?;
        }
        writeln!(f, "  +-----------------+")?;
        writeln!(f, "    a b c d e f g h")?;
        writeln!(
            f,
            "Active: {:?}, Castling: {:?}, EP: {:?}, Halfmove: {}, Fullmove: {}, Hash: {:016x}",
            self.active_colour,
            self.castling_rights,
            self.en_passant.map(bit_scan).map(index_to_position),
            self.halfmove_clock,
            self.fullmove_clock,
            self.zobrist_hash
        )
    }
}

impl Game {
    pub fn to_fen(&self) -> String {
        let mut fen = String::new();

        // 1. Piece placement
        for rank in (0..8).rev() {
            let mut empty_count = 0;
            for file in 0..8 {
                let sq = rank * 8 + file;
                match self.squares[sq] {
                    Square::Empty => {
                        empty_count += 1;
                    }
                    Square::Occupied(idx) => {
                        if empty_count > 0 {
                            fen.push_str(&empty_count.to_string());
                            empty_count = 0;
                        }
                        let p = &self.pieces[idx];
                        let ch = p.piece_type.to_char(p.colour);
                        fen.push(ch);
                    }
                }
            }
            if empty_count > 0 {
                fen.push_str(&empty_count.to_string());
            }
            if rank > 0 {
                fen.push('/');
            }
        }

        // 2. Active colour
        fen.push(' ');
        fen.push(if self.active_colour == Colour::White {
            'w'
        } else {
            'b'
        });

        // 3. Castling rights
        fen.push(' ');
        let mut castling = String::new();
        if self.castling_rights.contains(CastlingRights::WHITEKINGSIDE) {
            castling.push('K');
        }
        if self
            .castling_rights
            .contains(CastlingRights::WHITEQUEENSIDE)
        {
            castling.push('Q');
        }
        if self.castling_rights.contains(CastlingRights::BLACKKINGSIDE) {
            castling.push('k');
        }
        if self
            .castling_rights
            .contains(CastlingRights::BLACKQUEENSIDE)
        {
            castling.push('q');
        }
        if castling.is_empty() {
            castling.push('-');
        }
        fen.push_str(&castling);

        // 4. En passant
        fen.push(' ');
        if let Some(ep_bit) = self.en_passant {
            fen.push_str(&index_to_position(bit_scan(ep_bit)));
        } else {
            fen.push('-');
        }

        // 5. Halfmove and fullmove clocks
        fen.push_str(&format!(" {} {}", self.halfmove_clock, self.fullmove_clock));
        fen
    }
}

pub fn split_on(s: &str, sep: char) -> (&str, &str) {
    for (i, item) in s.char_indices() {
        if item == sep {
            return (&s[0..i], &s[i + 1..]);
        }
    }
    (s, "")
}

#[allow(non_snake_case)]
pub fn read_FEN(fen: &str) -> Game {
    let parts: Vec<&str> = fen.split_whitespace().collect();
    if parts.is_empty() {
        panic!("Empty FEN string");
    }

    let mut pieces = Vec::with_capacity(32);
    let mut squares = vec![Square::Empty; 64];
    let mut piece_bbs = [[0u64; 6]; 2];
    let mut colour_bbs = [0u64; 2];
    let mut occupied = 0u64;

    let rows: Vec<&str> = parts[0].split('/').collect();
    if rows.len() != 8 {
        panic!("Invalid FEN: expected 8 ranks, got {}", rows.len());
    }

    for (rank_idx, row) in rows.iter().enumerate() {
        let rank = 7 - rank_idx;
        let mut file = 0;
        for ch in row.chars() {
            if let Some(digit) = ch.to_digit(10) {
                file += digit as usize;
            } else if let Some((piece_type, colour)) = PieceType::from_char(ch) {
                if file >= 8 {
                    panic!("File overflow in FEN rank {}: {}", rank, row);
                }
                let sq = rank * 8 + file;
                let bit = 1u64 << sq;
                let piece_idx = pieces.len();
                pieces.push(Piece {
                    position: bit,
                    colour,
                    piece_type,
                });
                squares[sq] = Square::Occupied(piece_idx);
                piece_bbs[colour.index()][piece_type.index()] |= bit;
                colour_bbs[colour.index()] |= bit;
                occupied |= bit;
                file += 1;
            } else {
                panic!("Invalid piece char in FEN: {}", ch);
            }
        }
    }

    let active_colour = if parts.len() > 1 {
        match parts[1] {
            "w" => Colour::White,
            "b" => Colour::Black,
            other => panic!("Unknown colour designator: '{}'", other),
        }
    } else {
        Colour::White
    };

    let mut castling_rights = CastlingRights::NONE;
    if parts.len() > 2 {
        for ch in parts[2].chars() {
            match ch {
                'K' => castling_rights |= CastlingRights::WHITEKINGSIDE,
                'Q' => castling_rights |= CastlingRights::WHITEQUEENSIDE,
                'k' => castling_rights |= CastlingRights::BLACKKINGSIDE,
                'q' => castling_rights |= CastlingRights::BLACKQUEENSIDE,
                '-' => (),
                other => panic!("Invalid char in castling rights: '{}'", other),
            }
        }
    }

    let en_passant = if parts.len() > 3 {
        match parts[3] {
            "-" => None,
            s => match position_to_bit(s) {
                Ok(bit) => Some(bit),
                Err(e) => panic!("{}", e),
            },
        }
    } else {
        None
    };

    let halfmove_clock = if parts.len() > 4 {
        parts[4].parse().unwrap_or(0)
    } else {
        0
    };

    let fullmove_clock = if parts.len() > 5 {
        parts[5].parse().unwrap_or(1)
    } else {
        1
    };

    let mut game = Game {
        pieces,
        squares,
        active_colour,
        castling_rights,
        en_passant,
        halfmove_clock,
        fullmove_clock,
        piece_bbs,
        colour_bbs,
        occupied,
        zobrist_hash: 0,
        history: Vec::with_capacity(128),
    };
    game.zobrist_hash = game.compute_zobrist();
    game
}
