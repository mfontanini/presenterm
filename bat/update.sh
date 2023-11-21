#!/bin/bash

set -e

if [ $# -ne 1 ]
then
  echo "Usage: $0 <bat-git-hash>"
  exit 1
fi

script_path=$(realpath "$0")
script_dir=$(dirname "$script_path")
git_hash=$1
clone_path=$(mktemp -d)
output_file="$script_dir/syntaxes.bin"
output_tag="$script_dir/syntaxes.git-hash"

echo "Cloning repo @ ${git_hash} into '$clone_path'"
git clone https://github.com/sharkdp/bat.git $clone_path
cd $clone_path
git reset --hard $git_hash

cp assets/syntaxes.bin $output_file
echo $git_hash > $output_tag

echo "syntaxes file copied to '$output_file'"

