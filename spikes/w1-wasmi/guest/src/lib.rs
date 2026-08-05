// SPDX-License-Identifier: Apache-2.0
//! Spike W1 guest — the representative per-frame app workload from PLAN.md:
//! build and commit a 300-node UI tree, classify 60 lines of Markdown and 60
//! of Fountain, emit style runs.
//!
//! Throwaway. See `../README.md` before reusing anything here.

#![no_std]

mod arena;
mod fountain;
mod markdown;
mod runs;
mod tree;

use runs::Runs;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    // A trap is the correct outcome here; the host is measuring, not
    // recovering. R-02-26 covers containment in the real runtime.
    core::arch::wasm32::unreachable()
}

#[link(wasm_import_module = "host")]
extern "C" {
    /// SPEC-02 §4 `ui.commit`.
    fn commit_tree(ptr: u32, len: u32) -> u32;
    /// SPEC-02 §5 `set-style-runs`.
    fn set_style_runs(ptr: u32, len: u32) -> u32;
}

const MD: &str = include_str!("../../corpus/sample.md");
const FOUNTAIN: &str = include_str!("../../corpus/sample.fountain");

/// Take the first `n` lines — the visible range, not the document (R-02-10).
fn window<'a>(src: &'a str, n: usize, out: &mut [&'a str; 64]) -> usize {
    let mut count = 0;
    for line in src.lines() {
        if count == n || count == out.len() {
            break;
        }
        out[count] = line;
        count += 1;
    }
    count
}

/// Build and commit a 300-node tree. Returns the node count so the host can
/// check the workload actually ran.
#[no_mangle]
pub extern "C" fn bench_tree() -> u32 {
    arena::reset();
    let mut enc = match tree::Encoder::new(320) {
        Some(e) => e,
        None => return 0,
    };
    tree::build(&mut enc);
    match enc.finish() {
        Some((ptr, len, count)) => {
            // SAFETY: ptr/len address this module's own linear memory.
            unsafe { commit_tree(ptr, len) };
            count
        }
        None => 0,
    }
}

/// Classify 60 lines of Markdown and push the style runs.
#[no_mangle]
pub extern "C" fn bench_markdown() -> u32 {
    arena::reset();
    let mut lines = [""; 64];
    let n = window(MD, 60, &mut lines);
    let mut r = match Runs::new() {
        Some(r) => r,
        None => return 0,
    };
    markdown::classify(&lines[..n], markdown::BlockCtx::start(), 0, &mut r);
    // SAFETY: as above.
    unsafe { set_style_runs(r.ptr(), r.byte_len()) };
    r.count()
}

/// Classify 60 lines of Fountain and push the style runs.
#[no_mangle]
pub extern "C" fn bench_fountain() -> u32 {
    arena::reset();
    let mut lines = [""; 64];
    let n = window(FOUNTAIN, 60, &mut lines);
    let mut r = match Runs::new() {
        Some(r) => r,
        None => return 0,
    };
    fountain::classify(&lines[..n], fountain::FountainCtx::start(), 0, &mut r);
    // SAFETY: as above.
    unsafe { set_style_runs(r.ptr(), r.byte_len()) };
    r.count()
}

/// One frame's worth of app work: the three above, as the supervisor would
/// drive them. This is the figure that matters against the 16.6 ms budget.
#[no_mangle]
pub extern "C" fn bench_frame() -> u32 {
    bench_tree() + bench_markdown() + bench_fountain()
}

/// Paired true negative (CLAUDE.md §3). An export that does no work and calls
/// no host function. If the harness reports fuel for this in the same order of
/// magnitude as `bench_frame`, it is measuring call overhead rather than the
/// workload, and the whole measurement is void.
#[no_mangle]
pub extern "C" fn bench_noop() -> u32 {
    0
}
