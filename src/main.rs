pub mod board;
pub mod eval;
pub mod movegen;
pub mod search;
pub mod types;
pub mod uci;

// Re-export all original types and functions so existing API remains intact
pub use board::{Game, UndoInfo, read_FEN, split_on};
pub use types::{
    COL_MAP, CastlingRights, Colour, MOD67TABLE, Move, Piece, PiecePosition, PieceType, Square,
    bit_scan, bit_to_position, index_to_position, index_to_posotion, position_to_bit,
};
pub use uci::UciHandler;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut handler = UciHandler::new();

    if args.len() > 1 {
        match args[1].as_str() {
            "perft" => {
                let depth = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(4);
                println!("Running perft depth {}...", depth);
                handler.run_perft(depth);
                return;
            }
            "eval" => {
                handler.handle_command("d");
                handler.handle_command("eval");
                return;
            }
            "bench" => {
                println!("Running engine benchmark...");
                handler.handle_command("ucinewgame");
                handler.handle_command("go depth 6");
                return;
            }
            _ => {}
        }
    }

    // Default: run UCI / interactive interface
    handler.run();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_conversion() {
        assert_eq!(position_to_bit("a1").unwrap(), 1u64 << 0);
        assert_eq!(position_to_bit("e4").unwrap(), 1u64 << 28);
        assert_eq!(position_to_bit("h8").unwrap(), 1u64 << 63);

        assert_eq!(bit_to_position(1u64 << 0).unwrap(), "a1");
        assert_eq!(bit_to_position(1u64 << 28).unwrap(), "e4");
        assert_eq!(bit_to_position(1u64 << 63).unwrap(), "h8");
    }

    #[test]
    fn test_initialise_matches_start_fen() {
        let game_init = Game::initialise();
        let game_fen = read_FEN("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");

        assert_eq!(game_init.active_colour, game_fen.active_colour);
        assert_eq!(game_init.castling_rights, game_fen.castling_rights);
        assert_eq!(game_init.en_passant, game_fen.en_passant);
        assert_eq!(game_init.halfmove_clock, game_fen.halfmove_clock);
        assert_eq!(game_init.fullmove_clock, game_fen.fullmove_clock);
        assert_eq!(game_init.to_fen(), game_fen.to_fen());
    }

    #[test]
    fn test_fen_roundtrip() {
        let fens = [
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
            "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
            "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
            "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
        ];

        for fen in fens {
            let game = read_FEN(fen);
            assert_eq!(game.to_fen(), fen);
        }
    }

    #[test]
    fn test_perft_startpos() {
        let mut game = Game::initialise();
        assert_eq!(game.perft(1), 20);
        assert_eq!(game.perft(2), 400);
        assert_eq!(game.perft(3), 8902);
        assert_eq!(game.perft(4), 197281);
    }

    #[test]
    fn test_perft_kiwipete() {
        let mut game =
            read_FEN("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1");
        assert_eq!(game.perft(1), 48);
        assert_eq!(game.perft(2), 2039);
        assert_eq!(game.perft(3), 97862);
    }

    #[test]
    fn test_perft_pos3() {
        let mut game = read_FEN("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1");
        assert_eq!(game.perft(1), 14);
        assert_eq!(game.perft(2), 191);
        assert_eq!(game.perft(3), 2812);
        assert_eq!(game.perft(4), 43238);
    }

    #[test]
    fn test_perft_pos4() {
        let mut game = read_FEN("r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1");
        assert_eq!(game.perft(1), 6);
        assert_eq!(game.perft(2), 264);
        assert_eq!(game.perft(3), 9467);
    }

    #[test]
    fn test_perft_pos5() {
        let mut game = read_FEN("rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8");
        assert_eq!(game.perft(1), 44);
        assert_eq!(game.perft(2), 1486);
        assert_eq!(game.perft(3), 62379);
    }

    #[test]
    fn test_search_finds_mate_in_one() {
        // Scholar's mate delivery: Qxf7#
        let mut game =
            read_FEN("r1bqkb1r/pppp1ppp/2n5/4p3/2B1n3/5Q2/PPPP1PPP/RNB1K1NR w KQkq - 0 4");
        let mut searcher = search::Searcher::new(&mut game);
        let limits = search::SearchLimits {
            max_depth: 3,
            time_limit_ms: Some(1000),
        };
        let (mv, score) = searcher.search(&limits);
        assert!(mv.is_some());
        assert_eq!(mv.unwrap().to_uci(), "f3f7");
        assert!(score > search::MATE_SCORE - 10);
    }

    asdfasdfasdfasdfasddsdfs
}
