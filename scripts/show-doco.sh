#!/usr/bin/env bash
BASE=$(git rev-parse --show-toplevel)
cd $BASE
echo "building doco..."
cargo  doc  --document-private-items --all --open &
