#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Spike W1 — build the guest, run the harness, and check that fuel is
# identical across two host architectures.
#
# The cross-architecture run is the evidence for the portability claim in the
# finding: if fuel depended on the host CPU, these two columns would differ.
set -euo pipefail
cd "$(dirname "$0")"

SAMPLES_NATIVE="${1:-2000}"
SAMPLES_CROSS="${2:-200}"     # qemu is slow; wall time from it is meaningless anyway

echo "==> building guest (wasm32-unknown-unknown, release)"
( cd guest && cargo build --release --target wasm32-unknown-unknown )

# The finding's fuel figures are a property of these exact bytes. If a
# toolchain change moves them, the committed artifact and the finding have to
# move together — so fail loudly rather than silently measuring a new module.
BUILT=guest/target/wasm32-unknown-unknown/release/w1_guest.wasm
echo "==> checking the built guest against the committed artifact"
if ! sha256sum -c artifacts/w1_guest.wasm.sha256 --status 2>/dev/null \
   || ! cmp -s "$BUILT" artifacts/w1_guest.wasm; then
  # Three distinct values, because three different things can drift: the
  # toolchain (built != artifact), the artifact file itself (artifact !=
  # recorded), or the record (recorded stale).
  echo "GUEST DRIFT: the module, the artifact, and its recorded hash disagree."
  echo "  built now:      $(sha256sum "$BUILT" | cut -d' ' -f1)"
  echo "  artifact file:  $(sha256sum artifacts/w1_guest.wasm | cut -d' ' -f1)"
  echo "  recorded hash:  $(cut -d' ' -f1 < artifacts/w1_guest.wasm.sha256)"
  echo "The fuel figures in FINDING-W1 no longer describe the built module."
  echo "Refresh the artifact AND the finding together, or pin the toolchain back."
  exit 1
fi
echo "    ok — module matches the one FINDING-W1 measured"

echo "==> native run (x86_64)"
cargo build --release -q
./target/release/w1-host "$SAMPLES_NATIVE" | tee /tmp/w1-native.txt

if command -v qemu-riscv64-static >/dev/null 2>&1 \
   && rustup target list --installed | grep -q riscv64gc-unknown-linux-gnu; then
  echo
  echo "==> cross run (riscv64 under qemu — fuel only, wall time is emulated)"
  cargo build --release -q --target riscv64gc-unknown-linux-gnu
  qemu-riscv64-static -L /usr/riscv64-linux-gnu \
    ./target/riscv64gc-unknown-linux-gnu/release/w1-host "$SAMPLES_CROSS" \
    | tee /tmp/w1-cross.txt

  # Compare the export/ret/fuel/first-call columns only.
  cut -c1-45 /tmp/w1-native.txt | sed -n '5,10p' > /tmp/w1-native.fuel
  cut -c1-45 /tmp/w1-cross.txt  | sed -n '5,10p' > /tmp/w1-cross.fuel
  echo
  if diff -u /tmp/w1-native.fuel /tmp/w1-cross.fuel; then
    echo "FUEL PORTABILITY: identical across x86_64 and riscv64."
  else
    echo "FUEL PORTABILITY: DIFFERS — the finding's central claim is wrong."
    exit 1
  fi
else
  echo
  echo "skipping cross run: needs qemu-user-static and the"
  echo "riscv64gc-unknown-linux-gnu target. Fuel portability NOT checked."
fi

# The device runner: compiles for the P4's target, and its logic is checked
# here. Neither is a substitute for running it on a board.
echo
echo "==> device runner: build for riscv32imafc + test the logic on host"
if rustup target list --installed | grep -q riscv32imafc-unknown-none-elf; then
  ( cd device && cargo build --release -q --target riscv32imafc-unknown-none-elf )
  echo "    builds for riscv32imafc-unknown-none-elf"
else
  echo "    SKIPPED: riscv32imafc-unknown-none-elf target not installed"
fi
( cd device && cargo test --release -q )
echo "    logic tests pass — but this has NEVER run on an ESP32-P4."
