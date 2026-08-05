// SPDX-License-Identifier: Apache-2.0
//! Style-run buffer. `set-style-runs` takes (start, len, token) triples over
//! the visible span (R-04-02).

use crate::arena;

pub const T_HEADING: u8 = 1;
pub const T_EMPHASIS: u8 = 2;
pub const T_STRONG: u8 = 3;
pub const T_CODE: u8 = 4;
pub const T_LINK: u8 = 5;
pub const T_LIST: u8 = 6;
pub const T_QUOTE: u8 = 7;
pub const T_SCENE: u8 = 8;
pub const T_CHARACTER: u8 = 9;
pub const T_DIALOGUE: u8 = 10;
pub const T_PAREN: u8 = 11;
pub const T_TRANSITION: u8 = 12;
pub const T_NOTE: u8 = 13;

/// (start: u32, len: u32, token: u8, pad: [u8; 3])
const RUN_LEN: usize = 12;
const MAX_RUNS: usize = 2048;

pub struct Runs {
    off: usize,
    count: usize,
}

impl Runs {
    pub fn new() -> Option<Self> {
        let off = arena::alloc(MAX_RUNS * RUN_LEN)?;
        Some(Self { off, count: 0 })
    }

    pub fn push(&mut self, start: u32, len: u32, token: u8) {
        if self.count >= MAX_RUNS || len == 0 {
            return;
        }
        let at = self.off + self.count * RUN_LEN;
        arena::write_u32(at, start);
        arena::write_u32(at + 4, len);
        arena::write_u8(at + 8, token);
        self.count += 1;
    }

    /// Setext headings classify the line *above* the rule, so the most recent
    /// run is rewritten rather than a new one appended.
    pub fn retag_previous(&mut self, token: u8) {
        if self.count < 2 {
            return;
        }
        let at = self.off + (self.count - 2) * RUN_LEN;
        arena::write_u8(at + 8, token);
    }

    pub fn ptr(&self) -> u32 {
        arena::addr(self.off)
    }

    pub fn byte_len(&self) -> u32 {
        (self.count * RUN_LEN) as u32
    }

    pub fn count(&self) -> u32 {
        self.count as u32
    }
}
