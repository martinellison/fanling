#!/usr/bin/env bash
export BUILD_TYPE="$1"
reset
export RUSTTYPE=debug
MACHINE=$(uname -n)
echo "machine is $MACHINE, setting machine-specific options"
export BADRUST=no
BUILDOPT=""
case $MACHINE in
edward) ;;
pinkipi) ;;
xiaosan)
    if [[ "$BUILD_TYPE" == "offline" ]]; then
        export BUILDOPT="--offline"
    fi
    ;;
starnova | xiaomading)
    export BADRUST=yes
    if [[ "$BUILD_TYPE" == "" ]]; then
        export BUILDOPT="--offline"
    fi
    ;;
esac
export BASE=$(git rev-parse --show-toplevel)
if [[ "$BASE" == "" ]]; then
    echo "need to be in the git repository"
    exit 1
fi
# --- platform-indepenent and PC build ---
# hack for view schema
JAVADIR=$BASE/taiposwig
cd $BASE
if [[ "$BADRUST" == "no" ]]; then
    echo "formatting code..."
    cargo fmt
fi
ALTFMT=$HOME/progs/rustfmt/target/release/rustfmt
if [[ -x $ALTFMT ]]; then
    for F in $(ls $BASE/*/src/*.rs); do
        $ALTFMT $F
    done
fi
if [[ $? != 0 ]]; then exit 1; fi
echo "building rust ($BUILDOPT) ..."
cargo build $BUILDOPT
if [[ $? != 0 ]]; then exit 1; fi
echo "cbindgen..."
CBINDGEN_TARG=$BASE/target/fanling-c-interface.h
rm -f fanling-c-interface.h
cbindgen fanling-c-interface/src/lib.rs --output $CBINDGEN_TARG --lang c
if [[ $? != 0 ]]; then
    echo 'cbindgen error'
    exit 1
fi
if [[ ! -f $CBINDGEN_TARG ]]; then
    echo "no cbindgen" $CBINDGEN_TARG
    exit 1
fi
export JAVALOC="/usr/lib/jvm/java"
# if [[ "$JAVA_HOME" != "" ]]; then export JAVALOC=$JAVA_HOME; fi
if [[ ! -d $JAVALOC ]]; then
    echo "java loc wrong" $JAVALOC
    exit 0
fi
if [[ ! -d $JAVADIR ]]; then mkdir $JAVADIR; fi
if [[ $? != 0 ]]; then exit 1; fi
RUSTEXDIR=$BASE/target/debug/
RUSTEX="$RUSTEXDIR/libfanling_c_interface.a"
if [[ ! -f $RUSTEX ]]; then
    echo "no so file" $RUSTEX
    exit 0
fi

cp $RUSTEX .
echo "swig..."
export SWIGDIR=$BASE/swig
if [[ ! -d $SWIGDIR ]]; then mkdir -p $SWIGDIR; fi
swig -outdir $JAVADIR -java -package taiposwig $SWIGDIR/taipo.i
if [[ $? != 0 ]]; then exit 1; fi
echo "c compile..., java home is " $JAVA_HOME " java loc is " ${JAVALOC}
JAVAINCL=${JAVA_HOME}/include
if [[ ! -d ${JAVAINCL} ]]; then
    echo "cannot find java includes" $JAVAINCL
    exit 9
fi
gcc -shared $SWIGDIR/taipo_wrap.c -o $SWIGDIR/libtaiposwig.so -I"$JAVAINCL" -I"$JAVAINCL/linux" -I $BASE -fPIC
if [[ $? != 0 ]]; then exit 0; fi
# echo "building java..."
# $JAVALOC/bin/javac $JAVADIR/*.java $SWIGDIR/test.java
# if [[ $? != 0 ]]; then echo 'java compile error'; exit 1; fi
# echo "running..."
# LD_LIBRARY_PATH=.:java $JAVALOC/bin/java test

# --- Android build ---

echo "building for Android..."

MACHINE=$(uname -n)
echo "machine is $MACHINE, setting machine-specific options"
case $MACHINE in
edward)
    export ANDROID_HOME=/home/martin/android
    ;;
pinkipi | xiaomading | starnova)
    printf "\033[1;31mcannot build for Android on ths platform\033[0m\n"
    exit 0
    ;;
xiaosan)
    export ANDROID_HOME=$HOME/work/android/sdk
    ;;
tarantula)
    export ANDROID_HOME=$HOME/Android/Sdk
    ;;
esac
export EMULATOR=$ANDROID_HOME/tools/emulator
export ADB=$ANDROID_HOME/platform-tools/adb
case $MACHINE in
tarantula)
    export NDK_HOME=$ANDROID_HOME/ndk/21.0.6113669/
    ;;
xiaosan)
    export NDK_HOME=$ANDROID_HOME/ndk/21.0.6113669
    ;;
*)
    export NDK_HOME=$ANDROID_HOME/ndk-bundle
    ;;
esac
if [[ ! -d $NDK_HOME ]]; then
    echo "cannot find NDK home" $NDK_HOME
    exit 0
fi

for ARCH in aarch64 x86_64 armv7; do
    echo "building for" $ARCH " ----------"
    case $ARCH in
    aarch64)
        export ARCHID=arm64-v8a
        export RUST_TARGET=aarch64-linux-android
        export ANDR_CLANG_ARCH=aarch64-linux-android
        # export TOOLS=arm64
        export JNIDIR=arm64-v8a
        export TOOLARCH=aarch64-linux-android
        ;;
    armv7)
        export ARCHID=armeabi-v7a
        export RUST_TARGET=armv7-linux-androideabi
        export ANDR_CLANG_ARCH=armv7a-linux-androideabi
        export JNIDIR=armeabi-v7a
        export TOOLARCH=arm-linux-androideabi
        ;;
    x86_64)
        export ARCHID=x86
        export RUST_TARGET=x86_64-linux-android
        export ANDR_CLANG_ARCH=x86_64-linux-android
        export JNIDIR=x86_64
        export TOOLARCH=x86_64-linux-android
        ;;
    esac
    export ANDLEV=26

    HOST_TAG=linux-x86_64
    export TOOLCHAIN=$NDK_HOME/toolchains/llvm/prebuilt/$HOST_TAG
    if [[ ! -d "$TOOLCHAIN" ]]; then
        echo "toolchain does not exist" $TOOLCHAIN "for" $ARCH
        exit 1
    fi
    export CC=$TOOLCHAIN/bin/$ANDR_CLANG_ARCH$ANDLEV-clang
    export ARDIR=$TOOLCHAIN/bin
    export AR=$ARDIR/$TOOLARCH-ar
    if [[ ! -f "$CC" ]]; then
        echo "c compiler does not exist" $CC "for" $ARCH
        exit 1
    fi
    if [[ ! -f "$AR" ]]; then
        echo "c archiver does not exist" $AR "for" $ARCH
        ls $ARDIR
        exit 1
    fi
    cargo build --target $RUST_TARGET -p fanling-c-interface $BUILDOPT
    if [[ $? != 0 ]]; then
        echo "build failed for $ARCH"
        exit 1
    fi
    RUSTEX_ANDR="$BASE/target/$RUST_TARGET/debug/libfanling_c_interface.a"
    if [[ ! -f $RUSTEX_ANDR ]]; then
        echo "no so file" $RUSTEX_ANDR
        exit 0
    fi
    cp $RUSTEX_ANDR $SWIGDIR
    echo "android c compile... "
    rm -f $SWIGDIR/libtaiposwig.so
    $CC -shared $SWIGDIR/taipo_wrap.c $SWIGDIR/libfanling_c_interface.a -lm -llog -lz -o $SWIGDIR/libtaiposwig.so -I"${JAVALOC}/include" -I"${JAVALOC}/include/linux" -I $BASE -fPIC
    if [[ $? != 0 ]]; then exit 1; fi
    ANDDIR=$BASE/Lowu/app/src/main
    ANDLIBDIR=$ANDDIR/jniLibs
    ANDLIBDIR64=$ANDLIBDIR/$JNIDIR
    ANDJAVADIR=$ANDDIR/java/taiposwig
    if [[ ! -d $ANDJAVADIR ]]; then mkdir -p $ANDJAVADIR; fi
    if [[ ! -d $ANDLIBDIR64 ]]; then mkdir -p $ANDLIBDIR64; fi
    cp $SWIGDIR/libtaiposwig.so $ANDLIBDIR64
    echo "copying generated java to" $ANDJAVADIR
    cp $JAVADIR/*.java $ANDJAVADIR/
    cd $BASE/Lowu
    echo $ARCH "built"
done
./gradlew assembleDebug
if [[ $? != 0 ]]; then exit 1; fi
APKDIR="$BASE/Lowu/app/build/outputs/apk/debug"
echo "looking for APK in" $APKDIR
ls $APKDIR
echo "build done."
