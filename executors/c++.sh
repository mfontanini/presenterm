#!/usr/bin/env bash

temp=$(mktemp)
g++ -std=c++20 -x c++ "$1" -o "$temp"
"$temp"
