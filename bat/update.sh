#!/usr/bin/env bash

set -e

if [ $# -ne 1 ]; then
    echo "Usage: $0 <bat-git-hash>"
    exit 1
fi

script_path=$(realpath "$0")
script_dir=$(dirname "$script_path")
git_hash=$1
clone_path=$(mktemp -d)

echo "Cloning repo @ ${git_hash} into '$clone_path'"
git clone https://github.com/sharkdp/bat.git "$clone_path"
cd "$clone_path"
git reset --hard "$git_hash"

cp assets/syntaxes.bin "$script_dir"
cp assets/themes.bin "$script_dir"

acknowledgements_file="$script_dir/acknowledgements.txt"
cp LICENSE-MIT "$acknowledgements_file"
zlib-flate -uncompress <assets/acknowledgements.bin >>"$acknowledgements_file"
echo "$git_hash" >"$script_dir/bat.git-hash"

echo "syntaxes/themes updated"
