#!/usr/bin/env bash

set -e

script_path=$(realpath "$0")
script_dir=$(dirname "$script_path")
clone_path=$(mktemp -d)

git_hash=$(cat "$script_dir/bat.git-hash")
echo "Cloning repo @ ${git_hash} into '$clone_path'"
git clone https://github.com/sharkdp/bat.git "$clone_path"
cd "$clone_path"
git reset --hard "$git_hash"

for file in syntaxes.bin themes.bin
do
  our_hash=$(sha256sum "$script_dir/$file" | cut -d " " -f1)
  their_hash=$(sha256sum "$clone_path/assets/$file" | cut -d " " -f 1)
  if [ "$our_hash" != "$their_hash" ]
  then
    echo "Unexpected hash for ${file}: should be ${their_hash}, is ${our_hash}"
    exit 1
  fi
done

echo "All hashes match"
