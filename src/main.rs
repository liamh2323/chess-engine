use std::{fmt::format, vec};

type PiecePosition = u64;

fn bit_to_position(bit: PiecePosition) -> Result<String, String> {
    if bit == 0 {
        return Err("No Piece Present!".to_string()); 
    }else {
        let onebit_index = bit_scan(bit);
        return Ok(index_to_position(onebit_index));
    }
}

static COL_MAP:  [char;8] = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'];

fn indext_to_posotion(index: usize) -> String {
   let column = index % 8;
   let row = index / 8 + 1;

    return format!("{}{}, COL_MAP[column], row)");
}

// fn bit_scan_simple(mut bit: u64) -> usize {
//     let mut leading_zeros = 0;
//     while bit & 1 ==0 {
//         bit >>=1;
//         leading_zeros +=1;
//     }
//     return leading_zeros;
// }

static MOD67TABLE: [usize; 67] = [
    64, 0, 1, 39, 2, 15, 40, 23,
    3, 12, 16, 59, 41, 19, 24, 54,
    4, 63, 13, 10, 17, 62, 60, 28,
    42, 30, 20, 51, 25, 44, 55, 47,
    5, 32, 64, 38, 14, 22, 11, 58,
    18, 53, 63, 9, 61, 27, 29, 50,
    43, 46, 31, 37, 21, 57, 52, 8,
    26, 49, 45, 36, 56, 7, 48, 35,
    6, 34, 33
];

fn bit_scan(bit: u64) -> usize{
    let remainder = bit % 67;
    return MOD67TABLE[remainder];
}
#[derive(Debug, PartialEq,Clone, Copy)]
enum Colour {
    White,
    Black
}

#[derive(Debug, PartialEq)]
enum PieceType {
    Pawn, 
    Rook,
    Knight,
    Bishop,
    Queen,
    King
}

#[derive(Debug, PartialEq)]
struct Piece {
    position: PiecePosition,
    colour: Colour,
    piece_type: PieceType
}

// either empty or occupied
#[derive(Debug, PartialEq)]
enum Square {
    Empty,
    Occupied(usize),
}

// Game type to own the data
struct Game {
    pieces: Vec<Piece>,
    squares: Vec<Square>,
}

impl Game {
    fn push_piece_and_square(&mut self, position: usize, colour: Colour, piece_type: PieceType, index: &mut usize){
        self.pieces.push(Piece {position: (1 as u64) << position,
                                colour: colour,
                                piece_type: piece_type });
        self.squares.push(Square::Occupied((*index)));
        *index +=1;

    }
    fn push_empty_square(&mut self){
        self.squares.push(Square::Empty);
    }

    fn initialise() -> Game {
        let mut game = Game {pieces: vec![], squares: vec![]};
        let mut piece_index = 0;
        let colour = Colour::White;

        game.push_piece_and_square(0, colour, 
                                    PieceType::Rook, &mut piece_index);
        game.push_piece_and_square(1, colour, 
                                    PieceType::Knight, &mut piece_index);
        game.push_piece_and_square(2, colour, 
                                    PieceType::Bishop, &mut piece_index);
        game.push_piece_and_square(3, colour, 
                                    PieceType::Queen, &mut piece_index);
        game.push_piece_and_square(4, colour, 
                                    PieceType::King, &mut piece_index);
        game.push_piece_and_square(5, colour, 
                                    PieceType::Bishop, &mut piece_index);
        game.push_piece_and_square(6, colour, 
                                    PieceType::Knight, &mut piece_index);
        game.push_piece_and_square(7, colour, 
                                    PieceType::Rook, &mut piece_index);  

        for i in 8..16 {
            game.push_piece_and_square (i,colour, 
                                        PieceType::Pawn, &mut piece_index);
        }
        for i in 16..48 {
            game.push_empty_square();
        }
        let colour = Colour::Black;

        for i in 48..56 {
            game.push_piece_and_square (i, colour,
                PieceType::Pawn, &mut piece_index);
            }
        let offset = 56;

        game.push_piece_and_square(0+ offset, colour, 
                                    PieceType::Rook, &mut piece_index);
        game.push_piece_and_square(1+ offset, colour, 
                                    PieceType::Knight, &mut piece_index);
        game.push_piece_and_square(2+ offset, colour, 
                                    PieceType::Bishop, &mut piece_index);
        game.push_piece_and_square(3+ offset, colour, 
                                    PieceType::Queen, &mut piece_index);
        game.push_piece_and_square(4+ offset, colour, 
                                    PieceType::King, &mut piece_index);
        game.push_piece_and_square(5+ offset, colour, 
                                    PieceType::Bishop, &mut piece_index);
        game.push_piece_and_square(6+ offset, colour, 
                                    PieceType::Knight, &mut piece_index);
        game.push_piece_and_square(7+ offset, colour, 
                                    PieceType::Rook, &mut piece_index);  

            
     }  

    fn to_string(&self) -> String {
        let mut board = "".to_owned();
        let mut temp = "".to_owned();

        for (i, square) in self.squares.iter().enumerate(){
            match square {
                Square::Empty => temp.push_str(&indext_to_posotion(i)),
                Square::Occupied(idx) => temp.push_str(&self.pieces[*idx].to_string()),
            }

            if (i +1) % 8 == 0 {
                temp.push_str("\n");
                board.insert_str(0, &temp);
                temp.clear();
            }
        }
        board.insert_str(0, &temp);

        return board;
    }
}

impl Piece {
    fn to_string(&self) ->String{
        let mut result= match self.piece_type {
            PieceType::Pawn => "p ",
            PieceType::Rook => "r ",
            PieceType::Knight => "n ",
            PieceType::Bishop => "b ",
            PieceType::Queen => "q ",
            PieceType::King => "k ",
        }.to_string();
        if self.colour == Colour::White {
            result.make_ascii_uppercase();
        }
        return result;
    }
}
fn main(){

}