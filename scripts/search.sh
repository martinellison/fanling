#!/usr/bin/env bash
TARG=$1
find -name *.rs | grep -v "/old/" | grep -v target | xargs grep -in --color "$1"
