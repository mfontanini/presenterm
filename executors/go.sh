#!/usr/bin/env bash

export GO111MODULE=off
tempdir=$(mktemp -d)
cd "$tempdir"
mv "$1" main.go
go run main.go
rm -rf "$tempdir"
