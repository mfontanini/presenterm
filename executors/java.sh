#!/usr/bin/env bash

tempdir=$(mktemp -d)
cd "$tempdir"
cp "$1" Main.java
java Main.java
rm -rf "$tempdir"
