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
        tarantula)
            export RUST_SRC_PATH="$(rustc --print sysroot)/lib/rustlib/src/rust/src"
            ;;
    esac
    if [[ ! -d $BASE/testfiles ]] ; then mkdir $BASE/testfiles ; fi
    export DATABASE_URL=$BASE/testfiles/testdb.db
    if [[ "$NETSPEED" == "fast" || $START_TYPE == "all" ]] ; then
        echo "updating environment..."
        rustup update
    fi
    echo "pulling from git..."
    git pull
    # git clean -fX
    if [[ "$NETSPEED" == "fast" || $START_TYPE == "all" ]] ; then
        echo "updating crates..."
        cargo upgrade --workspace
     #   cargo upgrade --aggressive
    fi
    echo "restoring old cargo"
    git checkout $BASE/fanling-engine/Cargo.toml 
    export PATH=$PATH:$BASE/scripts:$BASE/target/debug
    diesel database reset
    $BASE/scripts/db-rebuild.sh 
    
    if [[ "$NETSPEED" != "fast" && $START_TYPE != "all" ]] ; then
        echo "pulling new cargo files..."
        cargo fetch
    fi
    # $BASE/scripts/db-rebuild.sh 

    qgit&
    scripts/copy-ssh.sh&
    cargo fix --allow-dirty
    cargo fmt
    cargo +nightly fix -Z unstable-options --clippy --allow-dirty
    scripts/edit.sh
    $BASE/scripts/show-doco.sh &
fi
