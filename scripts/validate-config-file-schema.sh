#!/bin/bash

script_dir=$(dirname "$0")
root_dir="${script_dir}/../"

current_schema=$(mktemp)
cargo run -q -- --generate-config-file-schema >"$current_schema"

diff=$(diff --color=always -u "${root_dir}/config-file-schema.json" "$current_schema")
if [ $? -ne 0 ]; then
  echo "Config file JSON schema differs:"
  echo "$diff"
  exit 1
else
  echo "Config file JSON schema is up to date"
fi
