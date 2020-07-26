#!/usr/bin/env bash
export BASE=$(git rev-parse --show-toplevel)
if [[ "$BASE" == "" ]]
then
    echo "need to be in the git repository"
    exit 1
fi
cd $BASE
/work/android-studio/bin/studio.sh &
