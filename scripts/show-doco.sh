#!/usr/bin/env bash
BASE=$(git rev-parse --show-toplevel)
cd $BASE
echo "cleaning cargo workspace..."
cargo clean
echo "building doco..."
cargo doc  --document-private-items --workspace --open &
