#!/bin/sh
set -eu
drastic_url="https://github.com/steward-fu/nds/releases/download/v1.8/drastic-v1.8_miyoo.zip"
dist="${PWD}"/dist/.allium/migrations/0002-drastic-1.8/drastic.zip
wget "$drastic_url" -O "$dist"
