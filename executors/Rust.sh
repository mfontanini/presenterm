#!/usr/bin/env bash

temp=$(mktemp)
rustc --crate-name "presenterm_snippet" "$1" -o "$temp"
"$temp"
