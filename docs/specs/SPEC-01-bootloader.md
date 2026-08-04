# SPEC-01 — Brokkr: Rust Second-Stage Bootloader

**Status:** Draft v0.1.0
**Depends on:** SPEC-00
**Risk:** This is the highest-uncertainty component in the project (R-01).
Read §8 before estimating.

---

## 1. Scope

Brokkr is the second-stage bootloader, loaded by the ESP32-P4 mask ROM from
flash. It is written in Rust, is `no_std`, and performs **no heap allocation
at any point**. It is the only component permitted to run before the kernel.

Responsibilities:

1. Minimal clock and flash bring-up sufficient to read the partition table.
2. Map flash into the instruction/data address space (cache + MMU config).
3. Initialise PSRAM.
4. Read the partition table and select an OS slot (A/B).
5. Verify the selected image.
6. Load segments, honour the rollback counter, jump.
7. Provide a recovery entry path when both slots fail or a key is held.

**Out of scope:** USB DFU, network recovery, filesystem access, anything
involving the C6, and any user interface beyond a small status indication.

---

## 2. What the ROM does for us

The mask ROM performs initial clocking, selects a boot source, and loads the
second-stage image from flash at a fixed offset. Under ESP Secure Boot v2 the
ROM verifies the second-stage image signature against a key digest burned into
eFuse before executing it.

**Confidence: Medium.** The exact offset, the image header layout accepted by
the P4 ROM, and the secure-boot scheme available on this silicon must be
confirmed against the TRM and ESP-IDF source during Spike B1 rather than
assumed from other ESP32 variants. Differences between P4 and earlier parts in
this area are the most likely source of a lost week.

Practical consequence: Brokkr must produce an image in the **ESP image
format the ROM expects**, not an arbitrary ELF. `tools/xtask` owns that
conversion and it is part of the deliverable, not an afterthought.

---

## 3. Boot sequence

```
reset
 └─ ROM: clock init, verify + load Brokkr to SRAM, jump
     └─ Brokkr:
         1. init_early()      clocks, watchdog, UART (defmt over RTT or UART)
         2. init_flash()      SPI flash mode/frequency, read partition table
         3. init_cache_mmu()  map flash regions
         4. init_psram()      <-- R-01, the hard part
         5. select_slot()     A/B via boot control block + rollback counter
         6. verify_image()    hash + signature over the selected slot
         7. load_segments()   to SRAM / PSRAM per segment header
         8. mark_pending()    if this is a trial boot
         9. jump()            cache flush, then transfer control
```

Any failure between 5 and 8 falls back to the other slot. Failure of both
enters recovery (§7).

### R-01-01 — Allocation-free
`crates/boot` declares no allocator and does not depend on `alloc`. All
buffers are `const`-sized statics or stack. CI verifies by attempting a build
with a panicking allocator linked and confirming no allocation symbols.

### R-01-02 — Size ceiling
The Brokkr image is ≤ 48 KiB. Checked in CI against the linked artifact.
Rationale: it must fit the ROM's staging constraints with margin, and size
discipline here is a proxy for scope discipline.

### R-01-03 — Deterministic timing
No dependency on interrupt timing or on peripherals not initialised by Brokkr
itself. Interrupts remain masked throughout except where a driver requires a
polled completion flag.

---

## 4. PSRAM initialisation (R-01, the hard part)

PSRAM bring-up on the P4 requires configuring the PSRAM controller, running a
device-specific initialisation sequence for the installed part, calibrating
timing, and configuring cache to map PSRAM into the address space. On existing
ESP parts this logic lives in ESP-IDF and is nontrivial; the sequences are
part- and mode-dependent and failure modes include *appearing to work* while
returning corrupted data under load.

That last property is why this cannot be signed off on a smoke test.

### R-01-04 — PSRAM validation
After init, Brokkr runs a **destructive memory test** over the full PSRAM
range: address-uniqueness (write address at address), walking ones/zeros, and
a pseudo-random pattern pass with a fixed seed. On mismatch it refuses to boot
the OS and enters recovery with a distinguishable status code.

This is the paired true negative for PSRAM health: a test that passes only
because the region was never actually written is the exact failure this
guards against. The address-uniqueness pass must be verified to fail when run
against a deliberately mis-mapped region — write that test.

### R-01-05 — Mode is recorded
The achieved PSRAM clock, mode, and size are written into a boot info
structure passed to the kernel. The kernel logs them and the budget tooling
consumes them. A memory measurement whose PSRAM mode is unknown is not a
finding (CLAUDE.md §3).

---

## 5. Partition layout

`boards/devkit-p4-7in/partitions.csv` is the single source of truth. Initial
layout for a 32 MiB flash:

| Name | Type | Offset | Size | Notes |
|---|---|---:|---:|---|
| `bootloader` | — | 0x0000 | 64 KiB | Brokkr; offset fixed by ROM |
| `ptable` | — | 0x10000 | 4 KiB | partition table |
| `bootctl` | data | 0x11000 | 8 KiB | A/B state, rollback counter, trial flag |
| `os_a` | app | 0x20000 | 6 MiB | kernel image slot A |
| `os_b` | app | 0x620000 | 6 MiB | kernel image slot B |
| `recovery` | app | 0xC20000 | 1 MiB | minimal recovery image |
| `sysdata` | data | 0xD20000 | 512 KiB | eFuse-adjacent config, device identity |
| `l10n` | data | 0xDA0000 | 1 MiB | Fluent bundles |
| `dict` | data | 0xEA0000 | 8 MiB | DAWG + dictionary, mmap-style read |
| `applocal` | data | 0x16A0000 | remainder | installed app packages |

Documents live on SD, never in flash.

### R-01-06 — A/B with rollback protection
`bootctl` holds: active slot, a monotonic rollback counter per slot, a trial
flag, and a boot-attempt count. Brokkr increments the attempt count before
jumping. The kernel clears the trial flag once it reaches a healthy state
(SPEC-03 defines "healthy"). Three failed attempts reverts to the other slot.

### R-01-07 — Anti-rollback
An image whose embedded version counter is lower than the value recorded in
`bootctl` is refused. The counter only ever increases.

---

## 6. Image verification

### R-01-08 — Verification is mandatory and unconditional
Brokkr verifies the OS image before jumping, in every build configuration
including development builds. Development builds may use a development key;
they may not skip verification. A `#[cfg]` that disables verification will not
be accepted — it is exactly the flag that ships by accident.

- Integrity: SHA-256 over the image, compared against the header.
- Authenticity: signature over that digest, using the scheme the P4 secure
  boot implementation supports (RSA-PSS-3072 or ECDSA-P256 — **confirm in
  Spike B1**; do not assume parity with other ESP32 parts).

Note the split from ADR-S011: firmware signing is constrained by ROM and eFuse
and we take what the silicon gives us. Application package signing (Ed25519,
SPEC-02 §9) is a separate trust domain with a separate key and a pure-Rust
verifier we control.

### R-01-09 — Failure is silent to the attacker, loud to the user
Verification failure produces a fixed-duration path regardless of *where* it
failed (no timing oracle), a distinguishable on-screen or LED status, and a
log entry in `bootctl`. It never falls through to executing the image.

---

## 7. Recovery

Recovery is entered when: both slots fail verification, PSRAM validation
fails, the attempt counter is exhausted, or a designated key is held at reset.

The recovery image is a minimal build that can: display a status screen,
re-flash an OS slot from a file on SD, and dump the boot log to SD. It has no
network, no app runtime, and no filesystem write access beyond the log.

### R-01-10 — Recovery cannot be updated by the normal path
The `recovery` partition is written only by full factory flashing. An OTA-ish
update from SD may never target it. Otherwise a bad update destroys the escape
hatch.

---

## 8. Delivery strategy and the interim fallback

Per ADR-S010, the ESP-IDF C bootloader is a permitted interim for Phases 1–2.
This is a deliberate decoupling: the app layer, the ABI, and the UI are the
parts most likely to be wrong in ways that matter, and they must not be
blocked behind PSRAM register sequences.

Exit criteria for the interim (must all hold before Phase 2 closes):

1. Brokkr boots the kernel from slot A on real hardware.
2. PSRAM validation (R-01-04) passes, including its true-negative test.
3. A/B failover demonstrated by deliberately corrupting slot A.
4. Anti-rollback demonstrated by attempting to install a lower version.
5. Recovery entry demonstrated by both trigger paths.
6. `[PRESENCE]` Cold boot to first launcher frame measured on hardware with a
   stopwatch-grade external check, not just an internal timer.

If Spike B1 shows PSRAM init in Rust is not achievable within the phase
budget, the fallback is a **narrow C shim** for the PSRAM sequence only,
called from Rust, with everything else in Rust. Record as an ADR amendment.
Do not silently abandon the requirement, and do not let the shim grow.

---

## 9. Testing

| Test | Where | Notes |
|---|---|---|
| Partition table parser | host | fuzzed; hostile input from flash is possible |
| Image header parser | host | fuzzed |
| Signature verification vectors | host | known-good and known-bad, including truncated and length-extended |
| A/B state machine | host | exhaustive over the small state space |
| PSRAM memory test logic | host + device | true-negative case required (R-01-04) |
| Boot timing | device | `[PRESENCE]` for the wall-clock check |
| Corrupted-slot failover | device | physical flash corruption, not simulated |
