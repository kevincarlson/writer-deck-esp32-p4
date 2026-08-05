// SPDX-License-Identifier: Apache-2.0
//! Fountain element classification (R-04-07 shape).
//!
//! Fountain is context-sensitive in a narrow way: an all-caps line is a
//! character cue only if preceded by a blank line and followed by a non-blank
//! one. That two-sided dependency is why the block context carries the
//! previous line's blankness rather than only a block kind.

use crate::runs::{
    Runs, T_CHARACTER, T_DIALOGUE, T_NOTE, T_PAREN, T_SCENE, T_TRANSITION,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct FountainCtx {
    pub prev_blank: bool,
    pub in_dialogue: bool,
    pub in_title_page: bool,
}

impl FountainCtx {
    pub const fn start() -> Self {
        Self {
            prev_blank: true,
            in_dialogue: false,
            in_title_page: true,
        }
    }
}

pub fn classify(lines: &[&str], mut ctx: FountainCtx, base: u32, runs: &mut Runs) -> FountainCtx {
    let mut off = base;
    for (i, line) in lines.iter().enumerate() {
        let next_blank = lines.get(i + 1).map(|l| l.trim().is_empty()).unwrap_or(true);
        ctx = classify_line(line, next_blank, ctx, off, runs);
        off += line.len() as u32 + 1;
    }
    ctx
}

fn classify_line(
    line: &str,
    next_blank: bool,
    mut ctx: FountainCtx,
    off: u32,
    runs: &mut Runs,
) -> FountainCtx {
    let t = line.trim();
    let len = line.len() as u32;

    if t.is_empty() {
        ctx.prev_blank = true;
        ctx.in_dialogue = false;
        return ctx;
    }

    // Title page: key: value pairs before the first blank line.
    if ctx.in_title_page {
        if t.contains(':') {
            runs.push(off, len, T_NOTE);
            ctx.prev_blank = false;
            return ctx;
        }
        ctx.in_title_page = false;
    }

    // Boneyard / notes / synopsis / section — never printed.
    if t.starts_with("[[") || t.starts_with('=') || t.starts_with('#') {
        runs.push(off, len, T_NOTE);
        ctx.prev_blank = false;
        return ctx;
    }

    // Centered text.
    if t.starts_with('>') && t.ends_with('<') {
        runs.push(off, len, T_TRANSITION);
        ctx.prev_blank = false;
        return ctx;
    }

    // Forced transition (`>`) or the trailing "TO:" form.
    if t.starts_with('>') || (is_upper(t) && t.ends_with("TO:")) {
        runs.push(off, len, T_TRANSITION);
        ctx.prev_blank = false;
        ctx.in_dialogue = false;
        return ctx;
    }

    // Scene heading: forced with a leading period, or a known prefix.
    if t.starts_with('.') && !t.starts_with("..") || is_scene_prefix(t) {
        runs.push(off, len, T_SCENE);
        ctx.prev_blank = false;
        ctx.in_dialogue = false;
        return ctx;
    }

    if ctx.in_dialogue {
        if t.starts_with('(') && t.ends_with(')') {
            runs.push(off, len, T_PAREN);
        } else {
            runs.push(off, len, T_DIALOGUE);
        }
        ctx.prev_blank = false;
        return ctx;
    }

    // The context-sensitive case: all-caps, blank before, non-blank after.
    if ctx.prev_blank && !next_blank && is_upper(t) {
        runs.push(off, len, T_CHARACTER);
        ctx.prev_blank = false;
        ctx.in_dialogue = true;
        return ctx;
    }

    // Action: no run emitted, it takes the body token by default.
    ctx.prev_blank = false;
    ctx
}

fn is_scene_prefix(t: &str) -> bool {
    const P: [&str; 6] = ["INT.", "EXT.", "EST.", "INT/EXT", "I/E.", "INT ."];
    P.iter().any(|p| t.starts_with(p))
}

/// Uppercase in the Fountain sense: has at least one letter and no lowercase.
fn is_upper(t: &str) -> bool {
    let mut has_alpha = false;
    for c in t.chars() {
        if c.is_ascii_lowercase() {
            return false;
        }
        if c.is_ascii_uppercase() {
            has_alpha = true;
        }
    }
    has_alpha
}
