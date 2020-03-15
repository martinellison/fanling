#!/usr/bin/env bash
export START_TYPE="$1"
reset
export CARGO_NAME="martin"
export CARGO_EMAIL="m.e@acm.org"

export BASE=$(git rev-parse --show-toplevel)
if [[ "$BASE" == "" ]]
then
    echo "need to be in the git repository"
else
    cd $BASE
    MACHINE=`uname -n`
    echo "machine is $MACHINE, setting machine-specific options"
    RUSTTOOLS="+nightly"
    NETSPEED="fast"
    case $MACHINE in
        edward) ;;
        pinkipi) ;;
        xiaosan) ;;
        starnova|xiaomading)
            RUSTTOOLS="" 
            NETSPEED="slow"
            ;;
    esac
    export DATABASE_URL=$BASE/testfiles/testdb.db
    if [[ "$NETSPEED" == "fast" || $START_TYPE == "all" ]] ; then
        echo "updating environment..."
        rustup update
    fi
    echo "pulling from git..."
    git pull
    git clean -fX
    if [[ "$NETSPEED" == "fast" || $START_TYPE == "all" ]] ; then
        echo "updating crates..."
        cargo upgrade --all
     #   cargo upgrade --aggressive
    fi
    echo "restoring old cargo"
    git checkout $BASE/fanling-engine/Cargo.toml $BASE/taipo-git-control/Cargo.toml
    export PATH=$PATH:$BASE/scripts:$BASE/target/debug
    $BASE/scripts/show-doco.sh &
    
    if [[ "$NETSPEED" != "fast" && $START_TYPE != "all" ]] ; then
        echo "pulling new cargo files..."
        cargo fetch
    fi
    $BASE/scripts/db-rebuild.sh 

    qgit&
    scripts/edit.sh
    scripts/copy-ssh.sh&
    cargo fmt
fi
