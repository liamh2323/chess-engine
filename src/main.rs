mod board;
use board::Board;

fn main() {
    let b = Board::new();
    match b.squares[0] {
        Some(p) => println!("a1 has a piece"),
        None => println!("a1 is empty"),
    }
}