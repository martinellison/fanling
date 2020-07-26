#!/usr/bin/env bash
export BASE=$(git rev-parse --show-toplevel)
if [[ "$BASE" == "" ]]; then
    echo "need to be in the git repository"
    exit 1
fi
# export EMACS=emacs
# if [[ -x /bin/emacs-26.2 ]] ; then
#         export EMACS=/bin/emacs-26.2
# fi
# if [[ -x /usr/bin/emacs ]] ; then
#     export EMACS=/usr/bin/emacs
# fi
# $EMACS $BASE/*/src/*.rs $BASE/*/src/*/*.rs &
/usr/bin/code fanling10-vsc.code-workspace &
