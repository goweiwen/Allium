#!/bin/sh
set -eu
drastic_url="https://github.com/steward-fu/archives/releases/download/miyoo-mini-plus/drastic-v1.5.zip"
dist="${PWD}"/dist/.allium/migrations/0002-drastic/drastic.zip
wget "$drastic_url" -O "$dist"
