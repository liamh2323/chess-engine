use crate::types::{CastlingRights, Colour, PieceType};

pub struct ZobristTables {
    pub pieces: [[[u64; 64]; 6]; 2],
    pub castling: [u64; 16],
    pub en_passant: [u64; 8],
    pub side_to_move: u64,
}

const fn splitmix64(mut state: u64) -> (u64, u64) {
    state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z = z ^ (z >> 31);
    (z, state)
}

impl ZobristTables {
    pub const fn init() -> Self {
        let mut seed = 0x1234_5678_9ABC_DEF0u64;
        let mut pieces = [[[0u64; 64]; 6]; 2];
        let mut castling = [0u64; 16];
        let mut en_passant = [0u64; 8];

        let mut c = 0;
        while c < 2 {
            let mut pt = 0;
            while pt < 6 {
                let mut sq = 0;
                while sq < 64 {
                    let (val, next) = splitmix64(seed);
                    seed = next;
                    pieces[c][pt][sq] = val;
                    sq += 1;
                }
                pt += 1;
            }
            c += 1;
        }

        let mut i = 0;
        while i < 16 {
            let (val, next) = splitmix64(seed);
            seed = next;
            castling[i] = val;
            i += 1;
        }

        let mut f = 0;
        while f < 8 {
            let (val, next) = splitmix64(seed);
            seed = next;
            en_passant[f] = val;
            f += 1;
        }

        let (side_to_move, _) = splitmix64(seed);

        ZobristTables {
            pieces,
            castling,
            en_passant,
            side_to_move,
        }
    }

    #[inline]
    pub fn piece_key(&self, colour: Colour, piece_type: PieceType, sq: usize) -> u64 {
        self.pieces[colour.index()][piece_type.index()][sq]
    }

    #[inline]
    pub fn castling_key(&self, rights: CastlingRights) -> u64 {
        self.castling[(rights.bits() & 0x0F) as usize]
    }

    #[inline]
    pub fn ep_key(&self, ep_sq: usize) -> u64 {
        self.en_passant[ep_sq % 8]
    }
}

pub static ZOBRIST: ZobristTables = ZobristTables::init();
