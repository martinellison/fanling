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
    export PATH=$PATH:$BASE/scripts:$BASE/target/debug
    $BASE/scripts/show-doco.sh &
    
    if [[ "$NETSPEED" != "fast" && $START_TYPE != "all" ]] ; then
        echo "pulling new cargo files..."
        cargo fetch
    fi
    # echo "recreating database and getting (and hacking) database schema..."
    # diesel database reset
    # SCHEMA=$BASE/fanling-engine/src/search/schema.rs
    # diesel print-schema >$SCHEMA
    # sed -i -e "s/item_by_level_copy2/item_by_level/g" $SCHEMA
    $BASE/scripts/db-rebuild.sh 
fi
