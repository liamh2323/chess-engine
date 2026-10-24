pub mod board;
pub mod eval;
pub mod movegen;
pub mod search;
pub mod tt;
pub mod types;
pub mod uci;
pub mod zobrist;

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
        assert_eq!(game_init.zobrist_hash, game_fen.zobrist_hash);
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
    fn test_zobrist_incremental_consistency() {
        fn verify_tree(game: &mut Game, depth: usize) {
            assert_eq!(game.zobrist_hash, game.compute_zobrist());
            if depth == 0 {
                return;
            }
            let moves = game.generate_legal_moves();
            for mv in moves {
                let before_hash = game.zobrist_hash;
                let undo = game.make_move(&mv);
                verify_tree(game, depth - 1);
                game.unmake_move(&mv, &undo);
                assert_eq!(game.zobrist_hash, before_hash);
            }
        }

        let mut kiwipete =
            read_FEN("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1");
        verify_tree(&mut kiwipete, 2);

        let mut promo_pos =
            read_FEN("rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8");
        verify_tree(&mut promo_pos, 2);
    }

    #[test]
    fn test_pawn_promotions() {
        let mut game = read_FEN("8/P7/8/8/8/8/8/k6K w - - 0 1");
        let moves = game.generate_legal_moves();
        let promo_moves: Vec<_> = moves.iter().filter(|m| m.promotion.is_some()).collect();
        assert_eq!(promo_moves.len(), 4);

        let mut handler = UciHandler::new();
        handler.handle_command("position fen 8/P7/8/8/8/8/8/k6K w - - 0 1 moves a7a8q");
        assert_eq!(handler.game.to_fen(), "Q7/8/8/8/8/8/8/k6K b - - 0 1");
    }

    #[test]
    fn test_en_passant_and_castling() {
        // En passant execution
        let mut handler = UciHandler::new();
        handler.handle_command("position startpos moves e2e4 a7a6 e4e5 d7d5 e5d6");
        assert_eq!(
            handler.game.to_fen(),
            "rnbqkbnr/1pp1pppp/p2P4/8/8/8/PPPP1PPP/RNBQKBNR b KQkq - 0 3"
        );

        // Castling kingside and queenside
        let mut castle_game = read_FEN("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1");
        let legal = castle_game.generate_legal_moves();
        assert!(legal.iter().any(|m| m.to_uci() == "e1g1" && m.is_castling));
        assert!(legal.iter().any(|m| m.to_uci() == "e1c1" && m.is_castling));
    }

    #[test]
    fn test_draw_detection() {
        // 1. Insufficient material
        let k_vs_k = read_FEN("8/8/4k3/8/8/4K3/8/8 w - - 0 1");
        assert!(k_vs_k.is_insufficient_material());

        let kb_vs_k = read_FEN("8/8/4k3/8/8/4K1B1/8/8 w - - 0 1");
        assert!(kb_vs_k.is_insufficient_material());

        let kr_vs_k = read_FEN("8/8/4k3/8/8/4K1R1/8/8 w - - 0 1");
        assert!(!kr_vs_k.is_insufficient_material());

        // 2. Threefold repetition
        let mut handler = UciHandler::new();
        handler.handle_command(
            "position startpos moves g1f3 g8f6 f3g1 f6g8 g1f3 g8f6 f3g1 f6g8",
        );
        assert!(handler.game.is_threefold_repetition());
        assert!(handler.game.is_draw());

        // 3. Fifty-move rule
        let fifty = read_FEN("8/8/4k3/8/8/4K1R1/8/8 w - - 100 75");
        assert!(fifty.is_fifty_move_draw());
        assert!(fifty.is_draw());
    }

    #[test]
    fn test_checkmate_and_stalemate() {
        // Fool's mate
        let mut fools = read_FEN("rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 3");
        assert!(fools.generate_legal_moves().is_empty());
        assert!(fools.is_in_check(Colour::White));

        // Stalemate
        let mut stalemate = read_FEN("7k/5Q2/6K1/8/8/8/8/8 b - - 0 1");
        assert!(stalemate.generate_legal_moves().is_empty());
        assert!(!stalemate.is_in_check(Colour::Black));
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

    #[test]
    fn test_search_finds_mate_in_two() {
        // White to move and mate in 2: 1. Re8+ Rxe8 2. Rxe8#
        let mut game = read_FEN("r5k1/5ppp/8/8/8/8/4RPPP/4R1K1 w - - 0 1");
        let mut searcher = search::Searcher::new(&mut game);
        let limits = search::SearchLimits {
            max_depth: 4,
            time_limit_ms: Some(2000),
        };
        let (mv, score) = searcher.search(&limits);
        assert_eq!(mv.unwrap().to_uci(), "e2e8");
        assert!(score > search::MATE_SCORE - 10);
        assert!(searcher.pv_line().len() >= 2);
    }
}
