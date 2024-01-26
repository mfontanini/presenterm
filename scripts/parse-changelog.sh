#!/usr/bin/env bash

set -e

if [ $# -ne 1 ]; then
    echo "Usage: $0 <version>"
    exit 1
fi

version=$1

if ! grep "^# ${version}" CHANGELOG.md >/dev/null; then
    echo "Version ${version} not found in changelog"
    exit 1
fi

sed -n "/^# ${version}/,/^# /{/^# ${version}/p;/^# /b;p}" CHANGELOG.md
