// SPDX-License-Identifier: Apache-2.0
//! Bump arena over a static buffer.
//!
//! An app gets 1.0 MiB of linear memory by default (SPEC-00 §6) and the real
//! SDK will hand it an allocator. This spike avoids `alloc` entirely so the
//! fuel figure measures the workload rather than an allocator we have not
//! chosen yet — see the README's note on what that excludes.

use core::ptr::addr_of_mut;

/// 256 KiB: the R-02-07 encoded-tree cap, which is the largest single buffer
/// the workload needs.
const ARENA_LEN: usize = 256 * 1024;

static mut ARENA: [u8; ARENA_LEN] = [0; ARENA_LEN];
static mut USED: usize = 0;

/// Reset the bump pointer. Called at the start of each measured operation so
/// runs are independent.
pub fn reset() {
    // SAFETY: wasm32 is single-threaded and there is no reentrancy here.
    unsafe { USED = 0 }
}

/// Bump-allocate `n` bytes, returning the offset into the arena.
/// Returns `None` rather than trapping so the caller decides.
pub fn alloc(n: usize) -> Option<usize> {
    // SAFETY: as above.
    unsafe {
        let start = USED;
        let end = start.checked_add(n)?;
        if end > ARENA_LEN {
            return None;
        }
        USED = end;
        Some(start)
    }
}

/// Give back the tail of the last allocation. Used to size the node region
/// exactly so the string arena lands immediately after it.
pub fn truncate_to(off: usize) {
    // SAFETY: as above. Callers only ever shrink.
    unsafe {
        if off <= USED {
            USED = off;
        }
    }
}

/// Absolute address of an arena offset, for handing to the host.
pub fn addr(off: usize) -> u32 {
    // SAFETY: as above. The cast is the point — the host reads linear memory
    // at this address.
    unsafe { (addr_of_mut!(ARENA) as *mut u8).add(off) as u32 }
}

pub fn write(off: usize, bytes: &[u8]) {
    // SAFETY: as above. Callers pass offsets returned by `alloc`.
    unsafe {
        let base = addr_of_mut!(ARENA) as *mut u8;
        core::ptr::copy_nonoverlapping(bytes.as_ptr(), base.add(off), bytes.len());
    }
}

pub fn write_u8(off: usize, v: u8) {
    write(off, &[v]);
}

pub fn write_u16(off: usize, v: u16) {
    write(off, &v.to_le_bytes());
}

pub fn write_u32(off: usize, v: u32) {
    write(off, &v.to_le_bytes());
}
