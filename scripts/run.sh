#!/usr/bin/env bash

export BASE=$(git rev-parse --show-toplevel)
if [[ "$BASE" == "" ]]; then
    echo "need to be in the git repository"
    exit 0
fi
cd $BASE

echo 'running'

CONFIG=${1:-$BASE/config-live.json}
grep write_to_server $CONFIG
PFX=$2
TESTBIN=$BASE/target/debug/fanling10
if [[ ! -f $TESTBIN ]]; then
    echo "no executable binary" $TESTBIN
    exit 3
fi
CMD="$PFX $TESTBIN -c $CONFIG"
echo "running:" $CMD
RUST_BACKTRACE=1 $CMD
echo 'done'
