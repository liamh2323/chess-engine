use bitflags::bitflags;
use std::fmt;

pub type PiecePosition = u64;

pub static COL_MAP: [char; 8] = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'];

pub static MOD67TABLE: [usize; 67] = [
    64, 0, 1, 39, 2, 15, 40, 23, 3, 12, 16, 59, 41, 19, 24, 54, 4, 63, 13, 10, 17, 62, 60, 28, 42,
    30, 20, 51, 25, 44, 55, 47, 5, 32, 64, 38, 14, 22, 11, 58, 18, 53, 63, 9, 61, 27, 29, 50, 43,
    46, 31, 37, 21, 57, 52, 8, 26, 49, 45, 36, 56, 7, 48, 35, 6, 34, 33,
];

#[inline]
pub fn bit_scan(bit: u64) -> usize {
    if bit == 0 {
        return 64;
    }
    bit.trailing_zeros() as usize
}

pub fn index_to_position(index: usize) -> String {
    let column = index % 8;
    let row = index / 8 + 1;
    format!("{}{}", COL_MAP[column], row)
}

// Kept for backward compatibility with existing code
pub fn index_to_posotion(index: usize) -> String {
    index_to_position(index)
}

pub fn bit_to_position(bit: PiecePosition) -> Result<String, String> {
    if bit == 0 {
        Err("No Piece Present!".to_string())
    } else {
        let onebit_index = bit_scan(bit);
        Ok(index_to_position(onebit_index))
    }
}

pub fn position_to_bit(position: &str) -> Result<PiecePosition, String> {
    if position.len() != 2 {
        return Err("Invalid length".to_string());
    }
    let bytes = position.as_bytes();
    let byte0 = bytes[0];
    if !(b'a'..=b'h').contains(&byte0) {
        return Err("Invalid Column Char".to_string());
    }
    let column = (byte0 - b'a') as usize;

    let byte1 = bytes[1];
    let row = match (byte1 as char).to_digit(10) {
        Some(number) if (1..=8).contains(&number) => (number - 1) as usize,
        _ => return Err("Invalid row".to_string()),
    };

    let square_number = row * 8 + column;
    let bit = 1u64 << square_number;
    Ok(bit)
}

pub fn square_to_coord(sq: usize) -> (usize, usize) {
    (sq % 8, sq / 8) // (file, rank) 0..7
}

pub fn coord_to_square(file: usize, rank: usize) -> usize {
    rank * 8 + file
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum Colour {
    White,
    Black,
}

impl Colour {
    #[inline]
    pub const fn opponent(self) -> Self {
        match self {
            Colour::White => Colour::Black,
            Colour::Black => Colour::White,
        }
    }

    #[inline]
    pub const fn index(self) -> usize {
        match self {
            Colour::White => 0,
            Colour::Black => 1,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum PieceType {
    Pawn,
    Rook,
    Knight,
    Bishop,
    Queen,
    King,
}

impl PieceType {
    #[inline]
    pub const fn index(self) -> usize {
        match self {
            PieceType::Pawn => 0,
            PieceType::Rook => 1,
            PieceType::Knight => 2,
            PieceType::Bishop => 3,
            PieceType::Queen => 4,
            PieceType::King => 5,
        }
    }

    pub fn to_char(self, colour: Colour) -> char {
        let ch = match self {
            PieceType::Pawn => 'p',
            PieceType::Rook => 'r',
            PieceType::Knight => 'n',
            PieceType::Bishop => 'b',
            PieceType::Queen => 'q',
            PieceType::King => 'k',
        };
        if colour == Colour::White {
            ch.to_ascii_uppercase()
        } else {
            ch
        }
    }

    pub fn from_char(ch: char) -> Option<(PieceType, Colour)> {
        let colour = if ch.is_ascii_uppercase() {
            Colour::White
        } else {
            Colour::Black
        };
        let piece_type = match ch.to_ascii_lowercase() {
            'p' => PieceType::Pawn,
            'r' => PieceType::Rook,
            'n' => PieceType::Knight,
            'b' => PieceType::Bishop,
            'q' => PieceType::Queen,
            'k' => PieceType::King,
            _ => return None,
        };
        Some((piece_type, colour))
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Piece {
    pub position: PiecePosition,
    pub colour: Colour,
    pub piece_type: PieceType,
}

impl fmt::Display for Piece {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut result = match self.piece_type {
            PieceType::Pawn => "p ",
            PieceType::Rook => "r ",
            PieceType::Knight => "n ",
            PieceType::Bishop => "b ",
            PieceType::Queen => "q ",
            PieceType::King => "k ",
        }
        .to_string();
        if self.colour == Colour::White {
            result.make_ascii_uppercase();
        }
        write!(f, "{}", result)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Square {
    Empty,
    Occupied(usize),
}

bitflags! {
    pub struct CastlingRights: u8 {
        const NONE = 0;
        const WHITEKINGSIDE = 1 << 0;
        const WHITEQUEENSIDE = 1 << 1;
        const BLACKKINGSIDE = 1 << 2;
        const BLACKQUEENSIDE = 1 << 3;
        const ALL =
            Self::WHITEKINGSIDE.bits
            | Self::WHITEQUEENSIDE.bits
            | Self::BLACKKINGSIDE.bits
            | Self::BLACKQUEENSIDE.bits;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Move {
    pub from: usize,
    pub to: usize,
    pub promotion: Option<PieceType>,
    pub is_en_passant: bool,
    pub is_castling: bool,
}

impl Move {
    pub fn new(from: usize, to: usize) -> Self {
        Move {
            from,
            to,
            promotion: None,
            is_en_passant: false,
            is_castling: false,
        }
    }

    pub fn to_uci(&self) -> String {
        let from_str = index_to_position(self.from);
        let to_str = index_to_position(self.to);
        let promo_str = match self.promotion {
            Some(PieceType::Queen) => "q",
            Some(PieceType::Rook) => "r",
            Some(PieceType::Bishop) => "b",
            Some(PieceType::Knight) => "n",
            _ => "",
        };
        format!("{}{}{}", from_str, to_str, promo_str)
    }
}

impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_uci())
    }
}
