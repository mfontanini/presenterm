#!/usr/bin/env bash

tempdir=$(mktemp -d)
cd "$tempdir"
cp "$1" script.kts
kotlinc -script script.kts
rm -rf "$tempdir"
