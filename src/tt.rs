use crate::search::MATE_SCORE;
use crate::types::Move;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TTFlag {
    None,
    Exact,
    LowerBound,
    UpperBound,
}

#[derive(Debug, Clone, Copy)]
pub struct TTEntry {
    pub key: u64,
    pub depth: u8,
    pub flag: TTFlag,
    pub score: i32,
    pub best_move: Option<Move>,
}

impl Default for TTEntry {
    fn default() -> Self {
        TTEntry {
            key: 0,
            depth: 0,
            flag: TTFlag::None,
            score: 0,
            best_move: None,
        }
    }
}

pub struct TranspositionTable {
    entries: Vec<TTEntry>,
    mask: usize,
}

impl Default for TranspositionTable {
    fn default() -> Self {
        Self::new_mb(16)
    }
}

impl TranspositionTable {
    pub fn new_mb(size_mb: usize) -> Self {
        let entry_size = std::mem::size_of::<TTEntry>().max(1);
        let target_bytes = size_mb.clamp(1, 1024) * 1024 * 1024;
        let raw_count = (target_bytes / entry_size).max(1024);
        let count = raw_count.next_power_of_two() >> 1;
        let count = count.max(1024);

        TranspositionTable {
            entries: vec![TTEntry::default(); count],
            mask: count - 1,
        }
    }

    pub fn clear(&mut self) {
        self.entries.fill(TTEntry::default());
    }

    #[inline]
    fn index(&self, key: u64) -> usize {
        (key as usize) & self.mask
    }

    #[inline]
    fn score_to_tt(score: i32, ply: usize) -> i32 {
        if score > MATE_SCORE - 500 {
            score + ply as i32
        } else if score < -MATE_SCORE + 500 {
            score - ply as i32
        } else {
            score
        }
    }

    #[inline]
    fn score_from_tt(score: i32, ply: usize) -> i32 {
        if score > MATE_SCORE - 500 {
            score - ply as i32
        } else if score < -MATE_SCORE + 500 {
            score + ply as i32
        } else {
            score
        }
    }

    pub fn probe(
        &self,
        key: u64,
        depth: usize,
        alpha: i32,
        beta: i32,
        ply: usize,
    ) -> (Option<i32>, Option<Move>) {
        let entry = &self.entries[self.index(key)];
        if entry.flag == TTFlag::None || entry.key != key {
            return (None, None);
        }

        let best_move = entry.best_move;

        if (entry.depth as usize) >= depth {
            let score = Self::score_from_tt(entry.score, ply);
            match entry.flag {
                TTFlag::Exact => return (Some(score), best_move),
                TTFlag::LowerBound if score >= beta => return (Some(score), best_move),
                TTFlag::UpperBound if score <= alpha => return (Some(score), best_move),
                _ => {}
            }
        }

        (None, best_move)
    }

    pub fn probe_move(&self, key: u64) -> Option<Move> {
        let entry = &self.entries[self.index(key)];
        if entry.flag != TTFlag::None && entry.key == key {
            entry.best_move
        } else {
            None
        }
    }

    pub fn store(
        &mut self,
        key: u64,
        depth: usize,
        flag: TTFlag,
        score: i32,
        best_move: Option<Move>,
        ply: usize,
    ) {
        let idx = self.index(key);
        let existing = &self.entries[idx];

        let depth_u8 = depth.min(u8::MAX as usize) as u8;
        if existing.flag == TTFlag::None
            || existing.key == key
            || depth_u8 >= existing.depth
            || flag == TTFlag::Exact
        {
            let mv = best_move.or(if existing.key == key {
                existing.best_move
            } else {
                None
            });
            self.entries[idx] = TTEntry {
                key,
                depth: depth_u8,
                flag,
                score: Self::score_to_tt(score, ply),
                best_move: mv,
            };
        }
    }

    pub fn hashfull(&self) -> usize {
        let sample = self.entries.len().min(1000);
        if sample == 0 {
            return 0;
        }
        let used = self.entries[..sample]
            .iter()
            .filter(|e| e.flag != TTFlag::None)
            .count();
        (used * 1000) / sample
    }
}
