#!/bin/sh

# Trigger release action on new release

if [ -z "$1" ]; then
    echo "Please provide a version number."
    echo "Usages: $0 v[X.Y.Z]"
    exit 1
fi

version=$1

git tag "v$version"
git push origin "v$version"
