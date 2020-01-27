#!/usr/bin/env bash
# copy files to github for release
export BASE=$(git rev-parse --show-toplevel)
GITHUB=$HOME/extgit/fanling
cd $BASE
rm -rf $BASE/fanling-engine/testfiles*
rm -rf $BASE/fanling-engine/"??"
rm -rf $BASE/taipo-git-control/testfiles
FF=".gitignore Cargo.toml LICENSE README.md cbindgen.toml config-test.yaml config.yaml"
for F in $FF
do
    cp $F $GITHUB
done
DD="Lowu fanling-c-interface fanling-engine fanling-interface fanling10 migrations scripts swig taipo-git-control taiposwig"
for D in $DD
do
    cp -R $D $GITHUB
done

cd $GITHUB
git add $FF $DD
