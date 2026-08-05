// SPDX-License-Identifier: Apache-2.0
//! Builds a 300-node UI tree and encodes it for `commit`.
//!
//! WARNING: this encoding is NOT `crates/abi`'s codec and must not be copied
//! into it. It mirrors the *shape* described in SPEC-02 §4 — fixed 8-byte
//! header (type, key, child count, flags), strings as (offset, len) into a
//! per-frame arena appended to the tree — so that the fuel figure is
//! representative. Everything else about it is throwaway.

use crate::arena;

// Node kinds, a subset of the SPEC-02 §4 table sufficient for a writer screen.
const K_COLUMN: u8 = 1;
const K_ROW: u8 = 2;
const K_SCROLL: u8 = 3;
const K_LABEL: u8 = 4;
const K_TEXTVIEW: u8 = 5;
const K_BUTTON: u8 = 6;
const K_LIST: u8 = 7;
const K_LIST_ITEM: u8 = 8;
const K_DIVIDER: u8 = 9;

const HEADER_LEN: usize = 8;
/// (str_off: u32, str_len: u32) for nodes that carry text.
const TEXT_PAYLOAD_LEN: usize = 8;

/// Style tokens are referenced by index, never by value (R-02-06).
const T_BODY: u8 = 0;
const T_HEADING_1: u8 = 1;
const T_CAPTION: u8 = 3;

pub struct Encoder {
    node_off: usize,
    node_cursor: usize,
    strings: [u8; 4096],
    strings_len: usize,
    count: u32,
}

impl Encoder {
    pub fn new(capacity_nodes: usize) -> Option<Self> {
        let node_off = arena::alloc(capacity_nodes * (HEADER_LEN + TEXT_PAYLOAD_LEN))?;
        Some(Self {
            node_off,
            node_cursor: node_off,
            strings: [0; 4096],
            strings_len: 0,
            count: 0,
        })
    }

    fn intern(&mut self, s: &str) -> (u32, u32) {
        let start = self.strings_len;
        let bytes = s.as_bytes();
        let end = start + bytes.len();
        if end > self.strings.len() {
            return (0, 0);
        }
        self.strings[start..end].copy_from_slice(bytes);
        self.strings_len = end;
        (start as u32, bytes.len() as u32)
    }

    fn header(&mut self, kind: u8, key: u32, children: u16, flags: u8) {
        let at = self.node_cursor;
        arena::write_u8(at, kind);
        arena::write_u8(at + 1, flags);
        arena::write_u32(at + 2, key);
        arena::write_u16(at + 6, children);
        self.node_cursor = at + HEADER_LEN;
        self.count += 1;
    }

    pub fn container(&mut self, kind: u8, key: u32, children: u16) {
        self.header(kind, key, children, 0);
    }

    pub fn text_node(&mut self, kind: u8, key: u32, style: u8, s: &str) {
        let (off, len) = self.intern(s);
        self.header(kind, key, 0, style);
        let at = self.node_cursor;
        arena::write_u32(at, off);
        arena::write_u32(at + 4, len);
        self.node_cursor = at + TEXT_PAYLOAD_LEN;
    }

    /// Append the string arena after the nodes and return (ptr, len) for the
    /// host, matching "a per-frame string arena appended to the tree".
    pub fn finish(&mut self) -> Option<(u32, u32, u32)> {
        // The node region was allocated for the worst case; give the unused
        // tail back so the string arena lands immediately after the nodes,
        // which is what the host's (offset, len) resolution assumes.
        arena::truncate_to(self.node_cursor);
        let soff = arena::alloc(self.strings_len)?;
        if soff != self.node_cursor {
            return None;
        }
        arena::write(soff, &self.strings[..self.strings_len]);
        let total = (self.node_cursor - self.node_off) + self.strings_len;
        Some((arena::addr(self.node_off), total as u32, self.count))
    }
}

/// A writer screen: toolbar, outline list, editor, status bar. Node count is
/// tuned to 300, the figure PLAN.md specifies for this spike.
pub fn build(enc: &mut Encoder) {
    let mut key = 1u32;
    let mut k = || {
        key += 1;
        key
    };

    enc.container(K_COLUMN, k(), 4); // root

    // Toolbar: 12 buttons + 2 labels.
    enc.container(K_ROW, k(), 14);
    for i in 0..12 {
        enc.text_node(K_BUTTON, k(), T_BODY, TOOLBAR[i % TOOLBAR.len()]);
    }
    enc.text_node(K_LABEL, k(), T_CAPTION, "The Salt Road");
    enc.text_node(K_LABEL, k(), T_CAPTION, "Chapter 4");

    enc.container(K_DIVIDER, k(), 0);

    // Body: outline scroll + editor.
    enc.container(K_ROW, k(), 2);

    // Outline: a virtualised list, windowed range only (R-02-07 forbids
    // committing the whole document's outline).
    enc.container(K_SCROLL, k(), 1);
    enc.container(K_LIST, k(), 128);
    for i in 0..128 {
        let style = if i % 8 == 0 { T_HEADING_1 } else { T_BODY };
        enc.text_node(K_LIST_ITEM, k(), style, OUTLINE[i % OUTLINE.len()]);
    }

    // Editor column: the text-view binds a host TextView by handle (SPEC-02
    // §5) — the document text does not cross the boundary.
    enc.container(K_COLUMN, k(), 2);
    enc.container(K_TEXTVIEW, k(), 0);
    enc.container(K_ROW, k(), 140);
    for i in 0..140 {
        enc.text_node(K_LABEL, k(), T_CAPTION, GUTTER[i % GUTTER.len()]);
    }

    // Status bar. Sized so the whole tree is exactly 300 nodes:
    // 1 root + 15 toolbar + 1 divider + 1 row + 1 scroll + 129 list
    // + 1 column + 1 text-view + 141 gutter + 9 status.
    enc.container(K_ROW, k(), 8);
    for i in 0..8 {
        enc.text_node(K_LABEL, k(), T_CAPTION, STATUS[i % STATUS.len()]);
    }
}

const TOOLBAR: [&str; 6] = ["File", "Edit", "View", "Outline", "Focus", "Export"];
const OUTLINE: [&str; 5] = [
    "The Salt Works",
    "Causeway",
    "Maren at the ledger",
    "Eighty-nine",
    "Fade out",
];
const GUTTER: [&str; 4] = ["1", "24", "137", "1042"];
const STATUS: [&str; 3] = ["12,431 words", "Markdown", "Saved"];
