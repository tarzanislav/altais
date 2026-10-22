//! Deterministic edit traces: typing a long note, an editing session with
//! inserts, deletions and formatting, and imported notes.

/// One edit, in Unicode scalar positions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Op {
    /// Insert `text` before position `pos`.
    Insert {
        /// Position in the text.
        pos: usize,
        /// Inserted text.
        text: String,
    },
    /// Delete `len` characters starting at `pos`.
    Delete {
        /// Position in the text.
        pos: usize,
        /// Number of characters.
        len: usize,
    },
    /// Make `start..end` bold.
    Bold {
        /// First character.
        start: usize,
        /// One past the last character.
        end: usize,
    },
}

/// Small deterministic random generator (xorshift64*), so every run and
/// every engine sees exactly the same trace.
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    /// Creates a generator; equal seeds give equal sequences.
    pub fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    /// Next raw value.
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    /// A value in `0..n` (`n` > 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
}

/// Words in the interface languages, all within the Basic Multilingual Plane.
const WORDS: &[&str] = &[
    "výlet",
    "Štrbské",
    "Pleso",
    "ráno",
    "raňajky",
    "lístky",
    "batoh",
    "hory",
    "volajú",
    "poznámka",
    "zošit",
    "štítok",
    "úloha",
    "príloha",
    "vyhľadávanie",
    "dôležité",
    "zvýraznené",
    "nový",
    "riadok",
    "odsek",
    "čas",
    "miesto",
    "voľno",
    "deň",
    "týždeň",
    "mesiac",
    "plán",
    "nákup",
    "recept",
    "kniha",
    "příliš",
    "žluťoučký",
    "kůň",
    "úpěl",
    "ďábelské",
    "ódy",
    "árvíztűrő",
    "tükörfúrógép",
    "zażółć",
    "gęślą",
    "jaźń",
    "съешь",
    "ещё",
    "этих",
    "мягких",
    "булок",
    "笔记",
    "同步",
    "加密",
    "搜索",
    "the",
    "and",
    "note",
    "meeting",
    "draft",
    "project",
    "budget",
    "travel",
];

/// A word from the mixed vocabulary.
pub fn word(rng: &mut Rng) -> &'static str {
    WORDS[rng.below(WORDS.len())]
}

/// Typing `words` words one keystroke at a time; every keystroke is one
/// transaction, as in the editor. Paragraph breaks every ~40 words.
pub fn typing(words: usize, seed: u64) -> Vec<Vec<Op>> {
    let mut rng = Rng::new(seed);
    let mut pos = 0;
    let mut out = Vec::new();
    for i in 0..words {
        let separator = if i == 0 {
            ""
        } else if i % 40 == 0 {
            "\n"
        } else {
            " "
        };
        let w = word(&mut rng);
        for ch in separator.chars().chain(w.chars()) {
            out.push(vec![Op::Insert {
                pos,
                text: ch.to_string(),
            }]);
            pos += 1;
        }
    }
    out
}

/// Length in characters after applying a typing trace.
pub fn typed_len(trace: &[Vec<Op>]) -> usize {
    trace.len()
}

/// An editing session on a text of `len` characters: the cursor wanders
/// through the note; 70 % typing a word, 20 % deleting, 10 % making a range
/// bold. One transaction per edit.
pub fn editing(mut len: usize, edits: usize, seed: u64) -> Vec<Vec<Op>> {
    let mut rng = Rng::new(seed);
    let mut cursor = len / 2;
    let mut out = Vec::with_capacity(edits);
    for _ in 0..edits {
        // Mostly local moves, sometimes a jump elsewhere in the note.
        if rng.below(10) == 0 {
            cursor = rng.below(len + 1);
        } else {
            let step = rng.below(41) as isize - 20;
            cursor = (cursor as isize + step).clamp(0, len as isize) as usize;
        }
        let kind = rng.below(10);
        if kind < 7 || len < 30 {
            let text = format!("{} ", word(&mut rng));
            let n = text.chars().count();
            out.push(vec![Op::Insert { pos: cursor, text }]);
            len += n;
            cursor += n;
        } else if kind < 9 {
            let n = (1 + rng.below(20)).min(len - cursor.min(len));
            if n == 0 {
                continue;
            }
            out.push(vec![Op::Delete {
                pos: cursor,
                len: n,
            }]);
            len -= n;
        } else {
            let start = cursor.min(len.saturating_sub(1));
            let end = (start + 1 + rng.below(30)).min(len);
            out.push(vec![Op::Bold { start, end }]);
        }
    }
    out
}

/// Text of an imported note of about `words` words (one insert, as the
/// Evernote import does).
pub fn note_text(words: usize, rng: &mut Rng) -> String {
    let mut text = String::new();
    for i in 0..words {
        if i > 0 {
            text.push(if i % 40 == 0 { '\n' } else { ' ' });
        }
        text.push_str(word(rng));
    }
    text
}

/// Applies a trace to a plain string, as the reference result.
pub fn apply_to_string(text: &mut Vec<char>, ops: &[Op]) {
    for op in ops {
        match op {
            Op::Insert { pos, text: t } => {
                let tail = text.split_off(*pos);
                text.extend(t.chars());
                text.extend(tail);
            }
            Op::Delete { pos, len } => {
                text.drain(*pos..*pos + *len);
            }
            Op::Bold { .. } => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn traces_are_deterministic_and_in_range() {
        assert_eq!(typing(50, 7), typing(50, 7));
        let typed = typing(200, 1);
        let mut text = Vec::new();
        for tx in &typed {
            apply_to_string(&mut text, tx);
        }
        assert_eq!(text.len(), typed_len(&typed));
        let session = editing(text.len(), 2_000, 2);
        for tx in &session {
            for op in tx {
                match op {
                    Op::Insert { pos, .. } => assert!(*pos <= text.len()),
                    Op::Delete { pos, len } => assert!(pos + len <= text.len()),
                    Op::Bold { start, end } => assert!(start < end && *end <= text.len()),
                }
            }
            apply_to_string(&mut text, tx);
        }
    }
}
