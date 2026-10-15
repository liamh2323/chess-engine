use crate::types::*;
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
}

impl Game {
    pub fn push_piece_and_square(
        &mut self,
        position: usize,
        colour: Colour,
        piece_type: PieceType,
        index: &mut usize,
    ) {
        self.pieces.push(Piece {
            position: 1u64 << position,
            colour,
            piece_type,
        });
        self.squares.push(Square::Occupied(*index));
        *index += 1;
    }

    pub fn push_empty_square(&mut self) {
        self.squares.push(Square::Empty);
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

    pub fn occupied_bb(&self) -> u64 {
        let mut bb = 0u64;
        for p in &self.pieces {
            bb |= p.position;
        }
        bb
    }

    pub fn colour_bb(&self, colour: Colour) -> u64 {
        let mut bb = 0u64;
        for p in &self.pieces {
            if p.colour == colour {
                bb |= p.position;
            }
        }
        bb
    }

    pub fn piece_bb(&self, colour: Colour, piece_type: PieceType) -> u64 {
        let mut bb = 0u64;
        for p in &self.pieces {
            if p.colour == colour && p.piece_type == piece_type {
                bb |= p.position;
            }
        }
        bb
    }

    pub fn king_square(&self, colour: Colour) -> usize {
        for p in &self.pieces {
            if p.colour == colour && p.piece_type == PieceType::King && p.position != 0 {
                return bit_scan(p.position);
            }
        }
        64
    }

    pub fn make_move(&mut self, mv: &Move) -> UndoInfo {
        let moved_piece_idx = match self.squares[mv.from] {
            Square::Occupied(idx) => idx,
            Square::Empty => panic!("Illegal move from empty square: {}", mv.from),
        };
        let moved_piece_orig = self.pieces[moved_piece_idx];

        let mut captured = None;
        let mut ep_captured = None;

        if mv.is_en_passant {
            let cap_sq = if self.active_colour == Colour::White {
                mv.to - 8
            } else {
                mv.to + 8
            };
            let cap_idx = match self.squares[cap_sq] {
                Square::Occupied(idx) => idx,
                Square::Empty => panic!("En passant capture on empty square: {}", cap_sq),
            };
            ep_captured = Some((cap_sq, cap_idx, self.pieces[cap_idx]));
            self.pieces[cap_idx].position = 0;
            self.squares[cap_sq] = Square::Empty;
        } else if let Square::Occupied(cap_idx) = self.squares[mv.to] {
            captured = Some((cap_idx, self.pieces[cap_idx]));
            self.pieces[cap_idx].position = 0;
        }

        let undo = UndoInfo {
            moved_piece_idx,
            moved_piece_orig,
            captured,
            ep_captured,
            castling_rights: self.castling_rights,
            en_passant: self.en_passant,
            halfmove_clock: self.halfmove_clock,
        };

        // Move the piece
        self.squares[mv.from] = Square::Empty;
        self.squares[mv.to] = Square::Occupied(moved_piece_idx);
        self.pieces[moved_piece_idx].position = 1u64 << mv.to;

        if let Some(promo) = mv.promotion {
            self.pieces[moved_piece_idx].piece_type = promo;
        }

        // Handle castling rook movement
        if mv.is_castling {
            match mv.to {
                6 => {
                    // White kingside: rook h1 (7) -> f1 (5)
                    let r_idx = match self.squares[7] {
                        Square::Occupied(i) => i,
                        _ => panic!("Rook missing at 7 for castling"),
                    };
                    self.squares[7] = Square::Empty;
                    self.squares[5] = Square::Occupied(r_idx);
                    self.pieces[r_idx].position = 1u64 << 5;
                }
                2 => {
                    // White queenside: rook a1 (0) -> d1 (3)
                    let r_idx = match self.squares[0] {
                        Square::Occupied(i) => i,
                        _ => panic!("Rook missing at 0 for castling"),
                    };
                    self.squares[0] = Square::Empty;
                    self.squares[3] = Square::Occupied(r_idx);
                    self.pieces[r_idx].position = 1u64 << 3;
                }
                62 => {
                    // Black kingside: rook h8 (63) -> f8 (61)
                    let r_idx = match self.squares[63] {
                        Square::Occupied(i) => i,
                        _ => panic!("Rook missing at 63 for castling"),
                    };
                    self.squares[63] = Square::Empty;
                    self.squares[61] = Square::Occupied(r_idx);
                    self.pieces[r_idx].position = 1u64 << 61;
                }
                58 => {
                    // Black queenside: rook a8 (56) -> d8 (59)
                    let r_idx = match self.squares[56] {
                        Square::Occupied(i) => i,
                        _ => panic!("Rook missing at 56 for castling"),
                    };
                    self.squares[56] = Square::Empty;
                    self.squares[59] = Square::Occupied(r_idx);
                    self.pieces[r_idx].position = 1u64 << 59;
                }
                _ => {}
            }
        }

        // Update en-passant square
        if moved_piece_orig.piece_type == PieceType::Pawn
            && (mv.to as isize - mv.from as isize).abs() == 16
        {
            let ep_sq = (mv.from + mv.to) / 2;
            self.en_passant = Some(1u64 << ep_sq);
        } else {
            self.en_passant = None;
        }

        // Update castling rights
        self.castling_rights &= Self::castling_mask(mv.from) & Self::castling_mask(mv.to);

        // Update clocks
        if moved_piece_orig.piece_type == PieceType::Pawn
            || captured.is_some()
            || ep_captured.is_some()
        {
            self.halfmove_clock = 0;
        } else {
            self.halfmove_clock += 1;
        }

        if self.active_colour == Colour::Black {
            self.fullmove_clock += 1;
        }

        self.active_colour = self.active_colour.opponent();
        undo
    }

    pub fn unmake_move(&mut self, mv: &Move, undo: &UndoInfo) {
        self.active_colour = self.active_colour.opponent();
        if self.active_colour == Colour::Black {
            self.fullmove_clock -= 1;
        }
        self.halfmove_clock = undo.halfmove_clock;
        self.castling_rights = undo.castling_rights;
        self.en_passant = undo.en_passant;

        // Undo castling rook movement
        if mv.is_castling {
            match mv.to {
                6 => {
                    let r_idx = match self.squares[5] {
                        Square::Occupied(i) => i,
                        _ => panic!("Rook missing at 5"),
                    };
                    self.squares[5] = Square::Empty;
                    self.squares[7] = Square::Occupied(r_idx);
                    self.pieces[r_idx].position = 1u64 << 7;
                }
                2 => {
                    let r_idx = match self.squares[3] {
                        Square::Occupied(i) => i,
                        _ => panic!("Rook missing at 3"),
                    };
                    self.squares[3] = Square::Empty;
                    self.squares[0] = Square::Occupied(r_idx);
                    self.pieces[r_idx].position = 1u64 << 0;
                }
                62 => {
                    let r_idx = match self.squares[61] {
                        Square::Occupied(i) => i,
                        _ => panic!("Rook missing at 61"),
                    };
                    self.squares[61] = Square::Empty;
                    self.squares[63] = Square::Occupied(r_idx);
                    self.pieces[r_idx].position = 1u64 << 63;
                }
                58 => {
                    let r_idx = match self.squares[59] {
                        Square::Occupied(i) => i,
                        _ => panic!("Rook missing at 59"),
                    };
                    self.squares[59] = Square::Empty;
                    self.squares[56] = Square::Occupied(r_idx);
                    self.pieces[r_idx].position = 1u64 << 56;
                }
                _ => {}
            }
        }

        // Restore moved piece
        let moved_idx = undo.moved_piece_idx;
        self.pieces[moved_idx] = undo.moved_piece_orig;
        self.squares[mv.to] = Square::Empty;
        self.squares[mv.from] = Square::Occupied(moved_idx);

        // Restore captured pieces
        if let Some((cap_sq, cap_idx, orig_piece)) = undo.ep_captured {
            self.pieces[cap_idx] = orig_piece;
            self.squares[cap_sq] = Square::Occupied(cap_idx);
        } else if let Some((cap_idx, orig_piece)) = undo.captured {
            self.pieces[cap_idx] = orig_piece;
            self.squares[mv.to] = Square::Occupied(cap_idx);
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
            "Active: {:?}, Castling: {:?}, EP: {:?}, Halfmove: {}, Fullmove: {}",
            self.active_colour,
            self.castling_rights,
            self.en_passant.map(bit_scan).map(index_to_position),
            self.halfmove_clock,
            self.fullmove_clock
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
                let piece_idx = pieces.len();
                pieces.push(Piece {
                    position: 1u64 << sq,
                    colour,
                    piece_type,
                });
                squares[sq] = Square::Occupied(piece_idx);
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

    Game {
        pieces,
        squares,
        active_colour,
        castling_rights,
        en_passant,
        halfmove_clock,
        fullmove_clock,
    }
}
