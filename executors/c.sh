#!/usr/bin/env bash

temp=$(mktemp)
gcc -x c "$1" -o "$temp"
"$temp"
