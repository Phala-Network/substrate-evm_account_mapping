#!/usr/bin/env sh
set -e

# The following lines ensure we run from the project root
PROJECT_ROOT=$(dirname "$(readlink -f "$0")")
cd "$PROJECT_ROOT"

echo "*** Run benchmark for pallet-account_abstraction ***"

./target/release/node-template benchmark pallet \
  --pallet=pallet_account_abstraction \
  --extrinsic="*" \
  --chain=dev \
  --steps=50 \
  --repeat=50 \
  --no-storage-info \
  --no-median-slopes \
  --no-min-squares \
  --wasm-execution=compiled \
  --heap-pages=4096 \
  --output=./pallets/account_abstraction/src/weights.rs \
  --template=./pallet-weight-template.hbs
