#!/usr/bin/env bash
# copy files to github for release (for personal use)
RELEASE=$1
if [[ "$RELEASE" == "" ]]; then
    echo "need to specify release"
    exit 1
fi
export BASE=$(git rev-parse --show-toplevel)
GITHUB=$HOME/extgit/fanling
if [[ -f $GITHUB ]]; then
    rm $GITHUB
    echo "deleted file" $GITHUB
fi
if [[ ! -d $GITHUB ]]; then
    echo "no directory" $GITHUB
    exit 1
fi
cd $BASE
rm -rf $BASE/fanling-engine/testfiles*
rm -rf $BASE/fanling-engine/"??"
rm -rf $BASE/taipo-git-control/testfiles
FF=".gitignore Cargo.toml LICENSE README.md cbindgen.toml config-test.json config.json"
for F in $FF; do
    cp $F $GITHUB
done
DD="Lowu android-keys fanling-c-interface fanling-engine fanling-interface fanling10 migrations scripts swig taipo-git-control taiposwig"
for D in $DD; do
    cp -R $D $GITHUB
done
git commit -am "release $RELEASE"
git tag $RELEASE

cd $GITHUB
git add $FF $DD
git commit -am "release $RELEASE"
git tag $RELEASE
echo "need to do: git push --all"
