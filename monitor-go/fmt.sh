#!/usr/bin/env bash
set -e

# in case IDE/gopls doesn't work

goimports -w .
gofmt -s -w .
