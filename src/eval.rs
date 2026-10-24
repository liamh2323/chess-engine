use crate::board::Game;
use crate::movegen::{ATTACKS, bishop_attacks, rook_attacks};
use crate::types::*;

pub const PIECE_VALUES: [i32; 6] = [100, 500, 320, 330, 900, 20000];

const PASSED_PAWN_MG: [i32; 8] = [0, 5, 10, 20, 35, 60, 100, 0];
const PASSED_PAWN_EG: [i32; 8] = [0, 10, 20, 40, 70, 115, 180, 0];

pub struct EvalMasks {
    pub file_masks: [u64; 8],
    pub adjacent_files_masks: [u64; 8],
    pub passed_pawn_masks: [[u64; 64]; 2],
}

impl EvalMasks {
    pub const fn init() -> Self {
        let mut file_masks = [0u64; 8];
        let mut adjacent_files_masks = [0u64; 8];
        let mut passed_pawn_masks = [[0u64; 64]; 2];

        let mut f = 0;
        while f < 8 {
            let mut mask = 0u64;
            let mut r = 0;
            while r < 8 {
                mask |= 1u64 << (r * 8 + f);
                r += 1;
            }
            file_masks[f] = mask;
            f += 1;
        }

        let mut f = 0;
        while f < 8 {
            let mut adj = 0u64;
            if f > 0 {
                adj |= file_masks[f - 1];
            }
            if f < 7 {
                adj |= file_masks[f + 1];
            }
            adjacent_files_masks[f] = adj;
            f += 1;
        }

        let mut sq = 0;
        while sq < 64 {
            let file = sq % 8;
            let rank = sq / 8;
            let files = file_masks[file] | adjacent_files_masks[file];

            // White passed pawn mask: all ranks > rank
            let mut w_front = 0u64;
            let mut r = rank + 1;
            while r < 8 {
                w_front |= 0xFFu64 << (r * 8);
                r += 1;
            }
            passed_pawn_masks[0][sq] = files & w_front;

            // Black passed pawn mask: all ranks < rank
            let mut b_front = 0u64;
            let mut r = 0;
            while r < rank {
                b_front |= 0xFFu64 << (r * 8);
                r += 1;
            }
            passed_pawn_masks[1][sq] = files & b_front;

            sq += 1;
        }

        EvalMasks {
            file_masks,
            adjacent_files_masks,
            passed_pawn_masks,
        }
    }
}

pub static EVAL_MASKS: EvalMasks = EvalMasks::init();

// PeSTO's Piece-Square Tables (MG = Middlegame, EG = Endgame)
// Laid out from Rank 8 (top, 0..7) to Rank 1 (bottom, 56..63).
#[rustfmt::skip]
const MG_PAWN_TABLE: [i32; 64] = [
      0,   0,   0,   0,   0,   0,   0,   0,
     98, 134,  61,  95,  68, 126,  34, -11,
     -6,   7,  26,  31,  65,  56,  25, -20,
    -14,  13,   6,  21,  23,  12,  17, -23,
    -27,  -2,  -5,  12,  17,   6,  10, -25,
    -26,  -4,  -4, -10,   3,   3,  33, -12,
    -35,  -1, -20, -23, -15,  24,  38, -22,
      0,   0,   0,   0,   0,   0,   0,   0,
];

#[rustfmt::skip]
const EG_PAWN_TABLE: [i32; 64] = [
      0,   0,   0,   0,   0,   0,   0,   0,
    178, 173, 158, 134, 147, 132, 165, 187,
     94, 100,  85,  67,  56,  53,  82,  84,
     32,  24,  13,   5,  -2,   4,  17,  17,
     13,   9,  -3,  -7,  -7,  -8,   3,  -1,
      4,   7,  -6,   1,   0,  -5,  -1,  -8,
     13,   8,   8, -10,   7,   0,  12,   0,
      0,   0,   0,   0,   0,   0,   0,   0,
];

#[rustfmt::skip]
const MG_KNIGHT_TABLE: [i32; 64] = [
    -167, -89, -34, -49,  61, -97, -15, -107,
     -73, -41,  72,  36,  23,  62,   7,  -17,
     -47,  60,  37,  65,  84, 129,  73,   44,
      -9,  17,  19,  53,  37,  69,  18,   22,
     -13,   4,  16,  13,  28,  19,  21,   -8,
     -23,  -9,  12,  10,  19,  17,  25,  -16,
     -29, -53, -12,  -3,  -1,  18, -14,  -19,
    -105, -21, -58, -33, -17, -28, -19,  -23,
];

#[rustfmt::skip]
const EG_KNIGHT_TABLE: [i32; 64] = [
    -58, -38, -13, -28, -31, -27, -63, -99,
    -25,  -8, -25,  -2,  -9, -25, -24, -52,
    -24, -20,  10,   9,  -1,  -9, -19, -41,
    -17,   3,  22,  22,  22,  11,   8, -18,
    -18,  -6,  16,  25,  16,  17,   4, -18,
    -23,  -3,  -1,  15,  10,  -3, -20, -22,
    -42, -20, -10,  -5,  -2, -20, -23, -44,
    -29, -51, -23, -15, -22, -18, -50, -64,
];

#[rustfmt::skip]
const MG_BISHOP_TABLE: [i32; 64] = [
    -29,   4, -82, -37, -25, -42,   7,  -8,
    -26,  16, -18, -13,  30,  59,  18, -47,
    -16,  37,  43,  40,  35,  50,  37,  -2,
     -4,   5,  19,  50,  37,  37,   7,  -2,
     -6,  13,  13,  26,  34,  12,  10,   4,
      0,  15,  15,  15,  14,  27,  18,  10,
      4,  15,  16,   0,   7,  21,  33,   1,
    -33,  -3, -14, -21, -13, -12, -39, -21,
];

#[rustfmt::skip]
const EG_BISHOP_TABLE: [i32; 64] = [
    -14, -21, -11,  -8,  -7,  -9, -17, -24,
     -8,  -4,   7, -12,  -3, -13,  -4, -14,
      2,  -8,   0,  -1,  -2,   6,   0,   4,
     -3,   9,  12,   9,  14,  10,   3,   2,
     -6,   3,  13,  19,   7,  10,  -3,  -9,
    -12,  -3,   8,  10,  13,   3,  -7, -15,
    -14, -18,  -7,  -1,   4,  -9, -15, -27,
    -23,  -9, -23,  -5,  -9, -16,  -5, -17,
];

#[rustfmt::skip]
const MG_ROOK_TABLE: [i32; 64] = [
     32,  42,  32,  51,  63,   9,  31,  43,
     27,  32,  58,  62,  80,  67,  26,  44,
     -5,  19,  26,  36,  17,  45,  61,  16,
    -24, -11,   7,  26,  24,  35,  -8, -20,
    -36, -26, -12,  -1,   9,  -7,   6, -23,
    -45, -25, -16, -17,   3,   0,  -5, -33,
    -44, -16, -20,  -9,  -1,  11,  -6, -71,
    -19, -13,   1,  17,  16,   7, -37, -26,
];

#[rustfmt::skip]
const EG_ROOK_TABLE: [i32; 64] = [
     13,  10,  18,  15,  12,  12,   8,   5,
     11,  13,  13,  11,  -3,   3,   8,   3,
      7,   7,   7,   5,   4,  -3,  -5,  -3,
      4,   3,  13,   1,   2,   1,  -1,   2,
      3,   5,   8,   4,  -5,  -6,  -8, -11,
     -4,   0,  -5,  -1,  -7, -12,  -8, -16,
     -6,  -6,   0,   2,  -9,  -9, -11,  -3,
     -9,   2,   3,  -1,  -5, -13,   4, -20,
];

#[rustfmt::skip]
const MG_QUEEN_TABLE: [i32; 64] = [
    -28,   0,  29,  12,  59,  44,  43,  45,
    -24, -39,  -5,   1, -16,  57,  28,  54,
    -13, -17,   7,   8,  29,  56,  47,  57,
    -27, -27, -16, -16,  -1,  17,  -2,   1,
     -9, -26,  -9, -10,  -2,  -4,   3,  -3,
    -14,   2, -11,  -2,  -5,   2,  14,   5,
    -35,  -8,  11,   2,   8,  15,  -3,   1,
     -1, -18,  -9,  10, -15, -25, -48, -18,
];

#[rustfmt::skip]
const EG_QUEEN_TABLE: [i32; 64] = [
     -9,  22,  22,  27,  27,  19,  10,  20,
    -17,  20,  32,  41,  58,  25,  30,   0,
    -20,   6,   9,  49,  47,  35,  19,   9,
      3,  22,  24,  45,  57,  40,  57,  36,
    -18,  28,  19,  47,  31,  34,  39,  18,
    -16, -27,  15,   6,   9,  17,  10,   5,
    -22, -23, -30, -16, -16, -23, -36, -32,
    -33, -28, -22, -43,  -5, -32, -20, -41,
];

#[rustfmt::skip]
const MG_KING_TABLE: [i32; 64] = [
    -65,  23,  16, -15, -56, -34,   2,  13,
     29,  -1, -20,  -7,  -8,  -4, -38, -29,
     -9,  24,   2, -16, -20,   6,  22, -22,
    -17, -20, -12, -27, -30, -25, -14, -36,
    -49,  -1, -27, -39, -46, -44, -33, -51,
    -14, -14, -22, -46, -44, -30, -15, -27,
      1,   7,  -8, -64, -43, -16,   9,   8,
    -15,  36,  12, -54,   8, -28,  24,  14,
];

#[rustfmt::skip]
const EG_KING_TABLE: [i32; 64] = [
    -74, -35, -18, -18, -11,  15,   4, -17,
    -12,  17,  14,  17,  17,  38,  23,  11,
     10,  17,  23,  15,  20,  45,  44,  13,
     -8,  22,  24,  27,  26,  33,  26,   3,
    -18,  -4,  21,  24,  27,  23,   9, -11,
    -19,  -3,  11,  21,  23,  16,   7,  -9,
    -27, -11,   4,  13,  14,   4,  -5, -17,
    -53, -34, -21, -11, -28, -14, -24, -43,
];

impl Game {
    pub fn evaluate(&self) -> i32 {
        if self.is_insufficient_material() {
            return 0;
        }

        let mut mg_scores = [0i32; 2];
        let mut eg_scores = [0i32; 2];
        let mut game_phase = 0i32;

        let occ = self.occupied_bb();
        let all_pawns =
            self.piece_bb(Colour::White, PieceType::Pawn) | self.piece_bb(Colour::Black, PieceType::Pawn);

        for colour in [Colour::White, Colour::Black] {
            let c = colour.index();
            let opp = colour.opponent();
            let friendly_occ = self.colour_bb(colour);
            let friendly_pawns = self.piece_bb(colour, PieceType::Pawn);
            let enemy_pawns = self.piece_bb(opp, PieceType::Pawn);

            let mut mg = 0i32;
            let mut eg = 0i32;

            // 1. Pawns (Material + PST + Passed + Isolated + Doubled)
            let mut pawns = friendly_pawns;
            while pawns != 0 {
                let sq = bit_scan(pawns);
                pawns &= pawns - 1;
                let table_sq = if colour == Colour::White { sq ^ 56 } else { sq };
                mg += 100 + MG_PAWN_TABLE[table_sq];
                eg += 100 + EG_PAWN_TABLE[table_sq];

                let file = sq % 8;
                let rel_rank = if colour == Colour::White {
                    sq / 8
                } else {
                    7 - (sq / 8)
                };

                // Passed pawn
                if (EVAL_MASKS.passed_pawn_masks[c][sq] & enemy_pawns) == 0 {
                    mg += PASSED_PAWN_MG[rel_rank];
                    eg += PASSED_PAWN_EG[rel_rank];
                }

                // Isolated pawn
                if (EVAL_MASKS.adjacent_files_masks[file] & friendly_pawns) == 0 {
                    mg -= 12;
                    eg -= 18;
                }
            }

            // Doubled pawns penalty per file
            for f in 0..8 {
                let count = (friendly_pawns & EVAL_MASKS.file_masks[f]).count_ones();
                if count > 1 {
                    let extra = (count - 1) as i32;
                    mg -= 10 * extra;
                    eg -= 20 * extra;
                }
            }

            // 2. Knights (Material + PST + Mobility)
            let mut knights = self.piece_bb(colour, PieceType::Knight);
            while knights != 0 {
                let sq = bit_scan(knights);
                knights &= knights - 1;
                let table_sq = if colour == Colour::White { sq ^ 56 } else { sq };
                mg += 320 + MG_KNIGHT_TABLE[table_sq];
                eg += 320 + EG_KNIGHT_TABLE[table_sq];
                game_phase += 1;

                let mob = (ATTACKS.knight_attacks[sq] & !friendly_occ).count_ones() as i32;
                mg += (mob - 4) * 3;
                eg += (mob - 4) * 3;
            }

            // 3. Bishops (Material + PST + Bishop Pair + Mobility)
            let mut bishops = self.piece_bb(colour, PieceType::Bishop);
            if bishops.count_ones() >= 2 {
                mg += 30;
                eg += 50;
            }
            while bishops != 0 {
                let sq = bit_scan(bishops);
                bishops &= bishops - 1;
                let table_sq = if colour == Colour::White { sq ^ 56 } else { sq };
                mg += 330 + MG_BISHOP_TABLE[table_sq];
                eg += 330 + EG_BISHOP_TABLE[table_sq];
                game_phase += 1;

                let mob = (bishop_attacks(sq, occ) & !friendly_occ).count_ones() as i32;
                mg += (mob - 6) * 4;
                eg += (mob - 6) * 4;
            }

            // 4. Rooks (Material + PST + Open/Semi-Open File + Mobility)
            let mut rooks = self.piece_bb(colour, PieceType::Rook);
            while rooks != 0 {
                let sq = bit_scan(rooks);
                rooks &= rooks - 1;
                let table_sq = if colour == Colour::White { sq ^ 56 } else { sq };
                mg += 500 + MG_ROOK_TABLE[table_sq];
                eg += 500 + EG_ROOK_TABLE[table_sq];
                game_phase += 2;

                let file_mask = EVAL_MASKS.file_masks[sq % 8];
                if (file_mask & all_pawns) == 0 {
                    mg += 24;
                    eg += 14;
                } else if (file_mask & friendly_pawns) == 0 {
                    mg += 12;
                    eg += 8;
                }

                let mob = (rook_attacks(sq, occ) & !friendly_occ).count_ones() as i32;
                mg += (mob - 7) * 2;
                eg += (mob - 7) * 3;
            }

            // 5. Queens (Material + PST)
            let mut queens = self.piece_bb(colour, PieceType::Queen);
            while queens != 0 {
                let sq = bit_scan(queens);
                queens &= queens - 1;
                let table_sq = if colour == Colour::White { sq ^ 56 } else { sq };
                mg += 900 + MG_QUEEN_TABLE[table_sq];
                eg += 900 + EG_QUEEN_TABLE[table_sq];
                game_phase += 4;
            }

            // 6. King (Material + PST + Pawn Shield)
            let mut kings = self.piece_bb(colour, PieceType::King);
            while kings != 0 {
                let sq = bit_scan(kings);
                kings &= kings - 1;
                let table_sq = if colour == Colour::White { sq ^ 56 } else { sq };
                mg += 20000 + MG_KING_TABLE[table_sq];
                eg += 20000 + EG_KING_TABLE[table_sq];

                // Pawn shield bonus in middlegame
                let shield = (ATTACKS.king_attacks[sq] & friendly_pawns).count_ones() as i32;
                mg += shield * 8;
            }

            mg_scores[c] = mg;
            eg_scores[c] = eg;
        }

        let mg_score = mg_scores[0] - mg_scores[1];
        let eg_score = eg_scores[0] - eg_scores[1];

        // Tapered evaluation: total max game phase = 24
        let phase = game_phase.min(24);
        let score = (mg_score * phase + eg_score * (24 - phase)) / 24;

        let perspective_score = if self.active_colour == Colour::White {
            score
        } else {
            -score
        };

        // Small tempo bonus for the side to move
        perspective_score + 10
    }
}
