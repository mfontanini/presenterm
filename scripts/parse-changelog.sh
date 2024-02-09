#!/usr/bin/env bash

set -e

script_dir=$(dirname "$0")
root_dir="${script_dir}/../"

if [ $# -ne 1 ]; then
    echo "Usage: $0 <version>"
    exit 1
fi

version=$1
changelog="${root_dir}/CHANGELOG.md"

if ! grep "^# ${version}" "$changelog" >/dev/null; then
    echo "Version ${version} not found in changelog"
    exit 1
fi

releases=$(grep -e "^# " -n "$changelog")
version_line=$(echo "$releases" | grep "$version" | cut -d : -f 1)
next_line=$(echo "$releases" | grep "$version" -A 1 -m 1 | tail -n 1 | cut -d : -f 1)
let next_line=("$next_line" - 1)

sed -n "${version_line},${next_line}p" "$changelog" | tail -n +3
