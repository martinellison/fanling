#!/usr/bin/env bash
OPTS="$1"
reset
export BASE=$(git rev-parse --show-toplevel)
if [[ "$BASE" == "" ]]
then
    echo "need to be in the git repository"
    exit 1
fi
cd $BASE
RUST_BACKTRACE=1 cargo test $OPTS
echo "test complete"
