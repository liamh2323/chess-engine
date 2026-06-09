#[derive(Clone, Copy, PartialEq)]
pub enum Colour { White, Black }

#[derive(Clone, Copy, PartialEq)]
pub enum PieceType { Pawn, Knight, Bishop, Rook, Queen, King }

#[derive(Clone, Copy)]
pub struct Piece {
    pub colour: Colour,
    pub piece_type: PieceType,
}

pub struct Board {
    pub squares: [Option<Piece>; 64],
    pub side_to_move: Colour,
    pub en_passant: Option<u8>,
    pub castling: u8,
}

impl Board {
    pub fn new() -> Self {
        let mut squares = [None; 64];

        let back_rank = [
            PieceType::Rook, PieceType::Knight, PieceType::Bishop, PieceType::Queen,
            PieceType::King, PieceType::Bishop, PieceType::Knight, PieceType::Rook,
        ];

        for file in 0..8 {
            squares[file] = Some(Piece { colour: Colour::White, piece_type: back_rank[file] });
            squares[8 + file] = Some(Piece { colour: Colour::White, piece_type: PieceType::Pawn });
            squares[48 + file] = Some(Piece { colour: Colour::Black, piece_type: PieceType::Pawn });
            squares[56 + file] = Some(Piece { colour: Colour::Black, piece_type: back_rank[file] });
        }

        Board {
            squares,
            side_to_move: Colour::White,
            en_passant: None,
            castling: 0b1111,
        }
    }
}