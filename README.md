# Chess Engine

A fast, UCI-compatible chess engine written in Rust featuring bitboard move generation, Zobrist hashing, a transposition table, Principal Variation Search (PVS), and tapered PeSTO + positional evaluation.

## Features

- **Board Representation & Move Generation**:
  - Hybrid piece-list and incremental 64-bit bitboard representation (`piece_bbs`, `colour_bbs`, `occupied`).
  - Precomputed attack tables for knights, kings, pawns, and 8-direction sliding rays (`rook_attacks`, `bishop_attacks`, `queen_attacks`).
  - Full chess rules: castling (kingside/queenside), en passant, promotions, check/checkmate/stalemate detection.
  - Full draw detection: threefold repetition, fifty-move rule, and insufficient material (`K vs K`, `KN vs K`, `KB vs K`, same-coloured `KB vs KB`).
  - Incremental 64-bit Zobrist hashing with O(1) `make_move` / `unmake_move` restoration.

- **Search**:
  - Iterative deepening with aspiration windows.
  - Principal Variation Search (PVS) with triangular PV-table tracking.
  - Transposition Table (TT) with exact, lower-bound, and upper-bound entries and mate-distance score normalization.
  - Check extensions, Null-Move Pruning (NMP), Reverse Futility Pruning (RFP), and Late Move Reductions (LMR).
  - Quiescence search with check evasions, captures, and promotions.
  - Move ordering: TT/PV move, MVV-LVA captures, promotions, killer moves, and history heuristic.

- **Evaluation**:
  - PeSTO tapered middlegame/endgame piece-square tables.
  - Pawn structure evaluation: passed pawns, isolated pawns, doubled pawns.
  - Piece mobility (knights, bishops, rooks), bishop pair bonus, rook on open/semi-open file bonus, king pawn shield, and tempo bonus.

## Building & Running

```bash
# Build optimized release binary
cargo build --release

# Run test suite
cargo test

# Run perft to depth 5
cargo run --release -- perft 5

# Run engine benchmark
cargo run --release -- bench

# Run interactive / UCI loop
cargo run --release
```

## UCI & Interactive Commands

- `uci` — Initialize UCI protocol and list engine options (`Hash`, `Clear Hash`).
- `setoption name Hash value <MB>` — Resize transposition table (1–1024 MB).
- `setoption name Clear Hash` — Clear the transposition table.
- `isready` — Check engine readiness (`readyok`).
- `ucinewgame` — Reset board to starting position and clear hash table.
- `position [startpos | fen <FEN>] [moves <m1> <m2> ...]` — Set board position.
- `go [depth <D>] [movetime <MS>] [wtime <MS>] [btime <MS>] [winc <MS>] [binc <MS>] [movestogo <N>] [infinite] [perft <D>]` — Search or run perft.
- `d` / `board` / `print` — Print ASCII board, metadata, Zobrist hash, and FEN.
- `eval` — Show static evaluation from the side-to-move's perspective.
- `moves` — List all legal moves in the current position.
- `undo` — Undo the last move in interactive mode.
- `<move>` (e.g. `e2e4`, `e7e8q`) — Play a move interactively and let the engine respond.
