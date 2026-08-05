// SPDX-License-Identifier: Apache-2.0
//! Incremental Markdown classification over a visible range.
//!
//! Shaped after R-04-02: classification starts at a line boundary from a
//! cached block context and must not require whole-document context. The
//! cases that force the cache to exist — fenced blocks, link reference
//! definitions, setext headings — are all present in `corpus/sample.md`.

use crate::runs::{Runs, T_CODE, T_EMPHASIS, T_HEADING, T_LINK, T_LIST, T_QUOTE, T_STRONG};

/// The per-line cache entry that makes classification restartable.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct BlockCtx {
    pub fence: Fence,
    pub list_depth: u8,
    pub in_quote: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Fence {
    None,
    Backtick,
    Tilde,
    Indented,
}

impl BlockCtx {
    pub const fn start() -> Self {
        Self {
            fence: Fence::None,
            list_depth: 0,
            in_quote: false,
        }
    }
}

/// Classify `lines`, threading the block context. Returns the context after
/// the last line, which is what a real implementation would cache.
pub fn classify(lines: &[&str], mut ctx: BlockCtx, base: u32, runs: &mut Runs) -> BlockCtx {
    let mut off = base;
    for line in lines {
        let len = line.len() as u32;
        ctx = classify_line(line, ctx, off, runs);
        off += len + 1; // +1 for the newline
    }
    ctx
}

fn classify_line(line: &str, mut ctx: BlockCtx, off: u32, runs: &mut Runs) -> BlockCtx {
    let b = line.as_bytes();
    let indent = leading_spaces(b);
    let trimmed = &b[indent..];

    // A fence closes only with its own delimiter — the tilde fence in the
    // corpus must not be closed by the backtick form.
    match ctx.fence {
        Fence::Backtick => {
            if starts_with(trimmed, b"```") {
                ctx.fence = Fence::None;
            }
            runs.push(off, line.len() as u32, T_CODE);
            return ctx;
        }
        Fence::Tilde => {
            if starts_with(trimmed, b"~~~") {
                ctx.fence = Fence::None;
            }
            runs.push(off, line.len() as u32, T_CODE);
            return ctx;
        }
        Fence::Indented => {
            if indent >= 4 {
                runs.push(off, line.len() as u32, T_CODE);
                return ctx;
            }
            ctx.fence = Fence::None;
        }
        Fence::None => {}
    }

    if trimmed.is_empty() {
        ctx.list_depth = 0;
        ctx.in_quote = false;
        return ctx;
    }

    if starts_with(trimmed, b"```") {
        ctx.fence = Fence::Backtick;
        runs.push(off, line.len() as u32, T_CODE);
        return ctx;
    }
    if starts_with(trimmed, b"~~~") {
        ctx.fence = Fence::Tilde;
        runs.push(off, line.len() as u32, T_CODE);
        return ctx;
    }
    if indent >= 4 && ctx.list_depth == 0 {
        ctx.fence = Fence::Indented;
        runs.push(off, line.len() as u32, T_CODE);
        return ctx;
    }

    if trimmed[0] == b'>' {
        ctx.in_quote = true;
        runs.push(off, line.len() as u32, T_QUOTE);
        return ctx;
    }

    if trimmed[0] == b'#' {
        let n = count_prefix(trimmed, b'#');
        if n <= 6 && trimmed.get(n) == Some(&b' ') {
            runs.push(off, line.len() as u32, T_HEADING);
            return ctx;
        }
    }

    // Setext: the underline classifies the *previous* line. A from-scratch
    // pass and an incremental pass must agree here, which is why the golden
    // corpus comparison in Phase 5 is byte-for-byte.
    if is_setext_rule(trimmed) {
        runs.push(off, line.len() as u32, T_HEADING);
        runs.retag_previous(T_HEADING);
        return ctx;
    }

    if is_link_ref_def(trimmed) {
        runs.push(off, line.len() as u32, T_LINK);
        return ctx;
    }

    if let Some(marker) = list_marker(trimmed) {
        ctx.list_depth = (indent / 2) as u8 + 1;
        runs.push(off, marker as u32, T_LIST);
        inline(line, indent + marker, off, runs);
        return ctx;
    }

    inline(line, indent, off, runs);
    ctx
}

/// Inline spans within a line: code, strong, emphasis, links.
fn inline(line: &str, from: usize, off: u32, runs: &mut Runs) {
    let b = line.as_bytes();
    let mut i = from;
    while i < b.len() {
        match b[i] {
            b'`' => {
                if let Some(end) = find(b, i + 1, b'`') {
                    runs.push(off + i as u32, (end - i + 1) as u32, T_CODE);
                    i = end + 1;
                    continue;
                }
            }
            b'*' if i + 1 < b.len() && b[i + 1] == b'*' => {
                if let Some(end) = find2(b, i + 2, b'*', b'*') {
                    runs.push(off + i as u32, (end - i + 2) as u32, T_STRONG);
                    i = end + 2;
                    continue;
                }
            }
            b'*' => {
                if let Some(end) = find(b, i + 1, b'*') {
                    runs.push(off + i as u32, (end - i + 1) as u32, T_EMPHASIS);
                    i = end + 1;
                    continue;
                }
            }
            b'[' => {
                if let Some(end) = find(b, i + 1, b']') {
                    runs.push(off + i as u32, (end - i + 1) as u32, T_LINK);
                    i = end + 1;
                    continue;
                }
            }
            _ => {}
        }
        i += 1;
    }
}

fn leading_spaces(b: &[u8]) -> usize {
    let mut i = 0;
    while i < b.len() && b[i] == b' ' {
        i += 1;
    }
    i
}

fn count_prefix(b: &[u8], c: u8) -> usize {
    let mut i = 0;
    while i < b.len() && b[i] == c {
        i += 1;
    }
    i
}

fn starts_with(b: &[u8], p: &[u8]) -> bool {
    b.len() >= p.len() && &b[..p.len()] == p
}

fn find(b: &[u8], from: usize, c: u8) -> Option<usize> {
    let mut i = from;
    while i < b.len() {
        if b[i] == c {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn find2(b: &[u8], from: usize, c1: u8, c2: u8) -> Option<usize> {
    let mut i = from;
    while i + 1 < b.len() {
        if b[i] == c1 && b[i + 1] == c2 {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn is_setext_rule(b: &[u8]) -> bool {
    if b.is_empty() {
        return false;
    }
    let c = b[0];
    (c == b'=' || c == b'-') && b.iter().all(|&x| x == c) && b.len() >= 2
}

fn is_link_ref_def(b: &[u8]) -> bool {
    if b.first() != Some(&b'[') {
        return false;
    }
    match find(b, 1, b']') {
        Some(end) => b.get(end + 1) == Some(&b':'),
        None => false,
    }
}

/// Returns the marker width if this line opens a list item.
fn list_marker(b: &[u8]) -> Option<usize> {
    if b.len() >= 2 && (b[0] == b'-' || b[0] == b'*' || b[0] == b'+') && b[1] == b' ' {
        return Some(2);
    }
    let digits = b.iter().take_while(|c| c.is_ascii_digit()).count();
    if digits > 0 && b.get(digits) == Some(&b'.') && b.get(digits + 1) == Some(&b' ') {
        return Some(digits + 2);
    }
    None
}
