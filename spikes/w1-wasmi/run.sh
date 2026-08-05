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
