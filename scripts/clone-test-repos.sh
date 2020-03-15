#!/usr/bin/env bash
echo "cloning test git"
export BASE=$(git rev-parse --show-toplevel)
TESTGITDIR=$BASE/fanling-engine/testfiles2
CLONEDIR=$TESTGITDIR/clones
echo "test dir is " $TESTGITDIR ", clone dir is" $CLONEDIR
if [[ -d $CLONEDIR ]] ; then
    echo "removing" $CLONEDIR
    rm -rf $CLONEDIR
fi
mkdir -p $CLONEDIR
cd $CLONEDIR
#REPODIR=$CLONEDIR/test-local
git clone $TESTGITDIR/test-local
echo "done"

