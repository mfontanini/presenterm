#!/bin/bash

set -e

script_dir=$(dirname "$0")
root_dir=$(realpath "${script_dir}/../")

echo "Creating python env"
env_dir=$(mktemp -d)
trap 'rm -rf "${env_dir}"' EXIT
python -mvenv "${env_dir}/pyenv"
source "${env_dir}/pyenv/bin/activate"

echo "Installing presenterm-export==0.1.2"
pip install presenterm-export

echo "Running presenterm..."
rm -f "${root_dir}/examples/demo.pdf"
cargo run -q -- --export-pdf "${root_dir}/examples/demo.md"

if test -f "${root_dir}/examples/demo.pdf"
then
  echo "PDF file has been created"
  rm -f "${root_dir}/examples/demo.pdf"
else
  echo "PDF file does not exist"
  exit 1
fi

