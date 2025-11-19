#!/bin/bash

set -euo pipefail

script_dir=$(dirname "$0")
root_dir="${script_dir}/../"

current_schema=$(mktemp)
docker run \
  --rm \
  -v "${root_dir}:/tmp/workspace" \
  -w "/tmp/workspace" \
  rust:1.86 \
  cargo run --features json-schema -q -- --generate-config-file-schema >"${current_schema}"

cp "$current_schema" "${root_dir}/config-file-schema.json"
rm "$current_schema"
