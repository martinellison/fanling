#!/usr/bin/env bash
# copy keys to set up testing
# where to get keys from
KEY_SRC=$HOME/.ssh/id_rsa
# copy for local tests
cp $KEY_SRC* /tmp
# location of android adb
ADB=/work/android/sdk/platform-tools/adb
$ADB root
# location of keys in android
AND_BASE=/data/user/0/hk.jennyemily.work.lowu
AND_LOC=$AND_BASE/id_rsa
$ADB push $KEY_SRC $AND_LOC
$ADB push $KEY_SRC.pub $AND_LOC.pub
$ADB shell chown u0_a84:u0_a84 $AND_LOC*
$ADB shell chmod og-wr,u-w $AND_LOC*
$ADB shell ls -l $AND_BASE
echo "copied"
