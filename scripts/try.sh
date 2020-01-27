#!/usr/bin/env bash
CONFIG=${1:-$BASE/config.yaml}
PFX=$2
export BASE=$(git rev-parse --show-toplevel)
if [[ "$BASE" == "" ]]
then
    echo "need to be in the git repository"
    exit 1
fi
TESTBIN=$BASE/target/debug/fanling10
CMD="$PFX $TESTBIN -c $CONFIG"
echo "running:" $CMD
RUST_BACKTRACE=1 $CMD
